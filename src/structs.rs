//use core::slice::SlicePattern;
use std::net::{TcpStream, UdpSocket, Ipv4Addr};
use std::io::{Error, ErrorKind, Read, Result, Seek, SeekFrom, Write};
use std::fs::File;
use std::string::FromUtf8Error;
use std::sync::{Mutex, Arc, RwLock};
use std::{thread, time};
//use crate::structs;
use byteorder::{ByteOrder, NetworkEndian};

struct Message {
    command: u8,
    message_type: MessageType,
}

enum MessageType {
    Data (u16),
    Text (u8, [u8; 256]),
}

pub struct Station {
    song_path: String,
    udp_ports: Arc<RwLock<Vec<u16>>>,
}

impl Clone for Station {
    fn clone(&self) -> Self {
        Self {
            song_path: self.song_path.clone(),
            udp_ports: self.udp_ports.clone(),
        }
    }
}

impl Station {
    pub fn new(song_path: String,
           udp_ports: Vec<u16>) -> Result<Self> {
        if let Err(error) = File::open(&song_path) {
            return Err(error)
        }
        Ok(Station {
            song_path,
            udp_ports: Arc::new(RwLock::new(udp_ports)),
        })
    }
    fn get_song_path_len(&self) -> u8 {
        self.song_path.len() as u8
    }
}

impl Message {
    fn new(command: u8,
           data: u16,
           message_length: u8,
           message: [u8; 256]) -> Result<Self> {
        match &command {
            0 | 1 | 2 => {
                //Ok(Message.MessageType::Short(command, data))
                Ok(Message {
                    command,
                    message_type: MessageType::Data(data)
                })
            }
            3 | 4 => {
                Ok(Message {
                    command,
                    message_type: MessageType::Text(message_length, message)
                })
            }
            _ => {
                eprintln!("Received command other than 0-4, in Message::new()):
                          {}. Exiting now.", &command);
                std::process::exit(1)
            }
        }
    }
    fn send(&self, stream: Arc<Mutex<TcpStream>>) -> Result<()> {
        let buf: [u8; 258] = match &self.message_type {
            MessageType::Data(data) => {
                let mut arr_temp = [0_u8; 258];
                arr_temp[0] = self.command;
                arr_temp[1] = data.to_be_bytes()[0];
                arr_temp[2] = data.to_be_bytes()[1];
                let arr = arr_temp;
                arr
            }
            MessageType::Text(message_length, message) => {
                let mut arr_temp = [0_u8; 258];
                arr_temp[0] = self.command;
                arr_temp[1] = *message_length;
                for (i, ch) in message.iter().enumerate() {
                    arr_temp[i] = *ch;
                }
                let arr: [u8; 258] = arr_temp;
                arr
            }
        };
        stream.lock().unwrap().write_all(&buf);
        stream.lock().unwrap().flush();
        Ok(())
    }
    fn receive(stream: Arc<Mutex<TcpStream>>) -> Result<Self> {
        //let mut buf = Vec::new();
        let mut buf = [0_u8; 258];
        match stream.lock().unwrap().read_exact(&mut buf) {
            Ok(_n_bytes) => {
                match &buf[0] {
                    0 | 1 | 2 => {
                        //Ok(Message::Short(buf[0],
                        //               NetworkEndian::read_u16(&buf[1..3])))
                        Message::new(buf[0],
                                     NetworkEndian::read_u16(&buf[1..3]),
                                     0,
                                     [0_u8; 256])
                    }
                    3 | 4 => {
                        //Ok(Message::Long(buf[0],
                        //              buf[1],
                        //              buf[2..].to_vec()))
                        let mut message = [0_u8; 256];
                        for (i, ch) in buf[2..].iter().enumerate() {
                            message[i] = *ch;
                        }
                        Message::new(buf[0],
                                     0,
                                     buf[1],
                                     message)
                    }
                    _ => {
                        eprintln!("Received command other than 0-4 in /
                                   message::receive(): {}. Exiting /
                                   now.", &buf[0]);
                        std::process::exit(1)
                    }
                }
            }
            Err(error) => {
                eprintln!("Error returned while reading TcpStream");
                Err(error)
            }
        }
    }
    fn receive_and_expect(stream: Arc<Mutex<TcpStream>>,
                          expected_command: u8) -> Result<Self> {
        match Message::receive(stream) {
            Ok(message) => {
                if message.command == expected_command {
                    Ok(message)
                } else {
                    eprintln!("The message received had command type {}, /
                               instead of the expected {}. Exiting.",
                               &message.command,
                               &expected_command);
                    std::process::exit(1)
                }
            }
            Err(error) => Err(error),
        }
    }
}

pub fn interact_with_server(stream: Arc<Mutex<TcpStream>>,
                            client_udp_port: u16) -> Result<()> {
    // 1. send hello
    let hello: Message = Message::new(0, client_udp_port, 0, [0_u8; 256])?;
    let _ = hello.send(stream.clone());
    println!("sent hello");
    // 2. receive welcome
    let mut number_stations_temp: u16 = 65535;
    let welcome: Message = Message::receive_and_expect(stream.clone(), 2)?;
    println!("received welcome");
    if let MessageType::Data(number_stations) = welcome.message_type {
        // 3. save number_stations in local struct TODO
        number_stations_temp = number_stations;
        // 4. print required message
        println!("Welcome to Snowcast! The server has {} stations.",
                 &number_stations);
    }
    let number_stations: u16 = number_stations_temp;
    let stream_clone = stream.clone();
    thread::spawn(move || wait_for_announce(stream_clone));
    // 5. send set_station message in loop
    loop {
        println!("What station would you like to select? If you're done, \
                  press \"q\" to exit.");
        let mut input = String::new();
        let _ = std::io::stdin().read_line(&mut input);
        let input: Vec<String> = input.split_whitespace().map(String::from).collect();
        //println!("{:?}", input);
        let station_number = if input.len() == 1 && input[0] == "q" {
            std::process::exit(1);
        } else if input.len() != 1 || input[0].parse::<u16>().is_err() {
            eprintln!("Pick a station from 0 to {} or quit with \"q\".", &number_stations);
            continue// 'input
        } else {
            input[0].parse::<u16>().unwrap()
        };

        let set_station: Message = Message::new(
            1, station_number, 0, [0_u8; 256])?;
        let _ = set_station.send(stream.clone());
        //set_station(&stream, station_number)?;
        println!("You selected station {}.", &station_number);
    }
}

//TODO do something with received announce now
fn wait_for_announce(stream: Arc<Mutex<TcpStream>>) -> Option<Message> {
    loop {
        let mut buf = [0_u8; 258];
        let _ = stream.lock().unwrap().peek(&mut buf);
        if buf[0] == 3 {
            //stream.lock().unwrap().read_exact(&mut buf.unwrap())?;
            return Some(Message::receive_and_expect(stream.clone(), 3).unwrap())
        }
    }
}

pub fn handle_client(stream: Arc<Mutex<TcpStream>>,
                     song_path_vec: Vec<String>,
                     station_vec: Vec<Station>) -> Result<()> {
    // 1. receive hello
    let mut client_udp_port_temp: u16 = 65535;
    let hello: Message = Message::receive_and_expect(stream.clone(), 0)?;
    if let MessageType::Data(udp_port) = hello.message_type {
        // 2. save client_udp_port in local structure TODO
        client_udp_port_temp = udp_port;
    }
    let client_udp_port: u16 = client_udp_port_temp;

    // 3. reply with the number of stations available in a welcome
    let welcome = Message::new(2, song_path_vec.len() as u16, 0, [0_u8; 256])?;
    //let stream_clone = stream.clone();
    let _ = welcome.send(stream.clone());

    // 4. start a loop to wait for set station with client
    let mut prev_station_opt: Option<Station> = None;
    //let station_vec_lock = station_vec.lock().unwrap(); //can't do this bc
                                                          // it would always
                                                          // have the lock
    loop {
        //let stream_clone = stream.clone();
        let set_station = Message::receive_and_expect(stream.clone(), 1)?;
        //let station_vec_lock = station_vec.lock().unwrap();
        if let MessageType::Data(station_number) = set_station.message_type {
            // 5. receive a station number from client, and add udp port to a 
            // shareable struct that will be used in a udp_player loop
            //let station_vec_clone = station_vec.clone();
            // let station_vec_lock = station_vec.lock().unwrap();
            // let station_got = station_vec_lock.get(station_number as usize);
            //let station_vec_clone = station_vec.clone();
            let station_opt = Some(station_vec.get(station_number as usize).unwrap());
            println!("\n\n\nstuck ahead of stat\n\n\n");
            if let Some(station) = station_opt {
                station.udp_ports.write().unwrap().push(client_udp_port);
            }
            println!("\n\n\nstuck ahead of prev_stat\n\n\n");
            if let Some(prev_station) = prev_station_opt {
                prev_station.udp_ports.write().unwrap().retain(|&x| x != client_udp_port);
            }
            //TODO add functionality to drop stream if client disconnects
            prev_station_opt = station_opt.cloned();
            println!("{}, {:?}", station_opt.unwrap().song_path, station_opt.unwrap().udp_ports.read().unwrap());
            //let announce = Message::receive_and_expect(stream.clone(), 3)?;
            let mut song_path_arr = [0_u8; 256];
            for (i, ch) in station_opt.unwrap()
                                      .song_path
                                      .as_bytes()
                                      .iter()
                                      .enumerate() {
                song_path_arr[i] = *ch;
            }
            // 6. announce song playing on new station
            let announce = Message::new(3, 0, station_opt.unwrap().song_path.len() as u8, song_path_arr)?;
            let _ = announce.send(stream.clone());
        }
    }
}

pub fn play_all_loops(server_name: Ipv4Addr,
                      server_udp_port: u16,
                      station_vec: Vec<Station>) -> Result<()> {
    for station in station_vec {
        let file = Arc::new(Mutex::new(File::open(&station.song_path).unwrap()));
        //let _ = play_song_loop(&file, &station.song_path, server_name, server_udp_port, &station.udp_ports);
        println!("spawning playing thread");
        thread::spawn(move || play_song_loop(file, station.song_path, server_name, server_udp_port, station.udp_ports));
    }
    Ok(())
}

fn play_song_loop(file: Arc<Mutex<File>>,
                  song_path: String,
                  server_name: Ipv4Addr,
                  server_udp_port: u16,
                  client_udp_port_vec: Arc<RwLock<Vec<u16>>>) -> Result<()> {
    println!("thread spawned!");
    let time_gap = std::time::Duration::from_micros(62500);
    let file_len = File::open(&song_path).unwrap().seek(std::io::SeekFrom::End(0)).unwrap();
    println!("\n\n\nfile_len: {:?}\n\n\n", file_len);
    loop {
        let progress = file.clone().lock().unwrap().stream_position().unwrap();
        let rel_pos = progress.clone() as f64 / file_len.clone() as f64;
        println!("rel_pos: {}", &rel_pos);
        println!("progress: {}", &progress);
        println!("file_len: {}", &file_len);
        let _ = play_song_chunk(file.clone(),
                                song_path.clone(),
                                server_name,
                                server_udp_port,
                                client_udp_port_vec.clone(),
        );
        thread::sleep(time_gap);
    }
}

fn play_song_chunk(file: Arc<Mutex<File>>,
                   song_path: String,
                   server_name: Ipv4Addr,
                   server_udp_port: u16,
                   client_udp_port_vec: Arc<RwLock<Vec<u16>>>) -> Result<()> {
    const BUF_LEN: u64 = 1024;
    let mut song_buf = [0_u8; BUF_LEN as usize];
    //match file.lock().unwrap().read_exact(&mut song_buf) {
    //    Ok(_) => {},
    //    Err(error) => match error.kind() {
    //        ErrorKind::UnexpectedEof => {
    //            let _ = file.lock().unwrap().rewind();
    //            println!("\n\n\n\n\n\n\nend of file reached!");
    //        }
    //        _ => {
    //            eprintln!("Error reading file: {}", song_path);
    //        }
    //    }
    //}
    let file_len = File::open(&song_path).unwrap().seek(std::io::SeekFrom::End(0)).unwrap();
    let current_pos = file.lock().unwrap().stream_position().unwrap();

    song_buf = if current_pos + BUF_LEN > file_len {
        let _ = file.lock().unwrap().read_exact(&mut song_buf);
        let _ = file.lock().unwrap().seek(SeekFrom::Start(0));
        song_buf
    } else {
        let _ = file.lock().unwrap().read_exact(&mut song_buf);
        song_buf
    };
    println!("read file in {}: {:?}", &song_path, std::time::SystemTime::now());
    let socket: UdpSocket = UdpSocket::bind(format!("{}:{}", server_name, server_udp_port))?;
    //let client_udp_port
    //let client_udp_port_vec_read = 'read: loop {
    //    let client_udp_port_vec_read_opt = Some(client_udp_port_vec.read().unwrap());
    //    if let Some(client_udp_port_vec_read) = client_udp_port_vec_read_opt {
    //        break 'read client_udp_port_vec_read
    //        //break 'read
    //    }
    //};
    println!("{:?}", client_udp_port_vec.read().unwrap());
    for udp in client_udp_port_vec.read().unwrap().iter() {
        println!("{}", &udp);
        let _ = socket.connect(format!("{}:{}", server_name, udp));
        let _ = socket.send(&song_buf);
    }
    Ok(())
}
