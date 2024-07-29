//use core::slice::SlicePattern;
use std::net::{TcpStream, UdpSocket, Ipv4Addr};
use std::io::{Error, ErrorKind, Read, Result, Seek, Write};
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
            if let Some(station) = station_opt {
                station.udp_ports.write().unwrap().push(client_udp_port);
            }
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
