//use core::slice::SlicePattern;
use std::net::Ipv4Addr;
use std::os::unix::process;
use std::process::exit;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UdpSocket};
use tokio::select;
use tokio::sync::{Mutex, RwLock};
use std::io::{Error, ErrorKind, Read, Result, Seek, SeekFrom, Write};
use std::fs::File;
//use std::string::FromUtf8Error;
use std::sync::Arc;
use std::{thread, time};
//use crate::structs;
use byteorder::{ByteOrder, NetworkEndian};
use std::sync::mpsc::{channel, Sender};

#[derive(Debug)]
struct Message {
    command: u8,
    message_type: MessageType,
}

#[derive(Debug)]
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
    async fn new(command: u8,
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
    async fn send(&self, stream: Arc<Mutex<TcpStream>>) -> Result<()> {
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
                    arr_temp[i + 2] = *ch;
                }
                let arr: [u8; 258] = arr_temp;
                arr
            }
        };
        println!("writing to stream!");
        let _ = stream.lock().await.write(&buf).await;
        let _ = stream.lock().await.flush().await;
        Ok(())
    }
    async fn receive(stream: Arc<Mutex<TcpStream>>) -> Result<Self> {
        //let mut buf = Vec::new();
        let mut buf = [0_u8; 258];
        match stream.lock().await.read_exact(&mut buf).await {
            Ok(_n_bytes) => {
                match &buf[0] {
                    0 | 1 | 2 => {
                        //Ok(Message::Short(buf[0],
                        //               NetworkEndian::read_u16(&buf[1..3])))
                        Message::new(buf[0],
                                     NetworkEndian::read_u16(&buf[1..3]),
                                     0,
                                     [0_u8; 256]).await
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
                                     message).await
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
    async fn receive_and_expect(stream: Arc<Mutex<TcpStream>>,
                                expected_command: u8) -> Result<Self> {
        match Message::receive(stream).await {
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
    async fn receive_and_expect2(stream: Arc<Mutex<TcpStream>>,
                                 expected_command: u8) -> Option<Self> {
        let mut buf = [0_u8; 258];
        let _ = stream.lock().await.peek(&mut buf).await;
        if buf[0] == expected_command {
            let mut buf = [0_u8; 258];
            let _ = stream.lock().await.read_exact(&mut buf).await;
            match &buf[0] {
                0 | 1 | 2 => {
                    Some(Message::new(buf[0],
                                      NetworkEndian::read_u16(&buf[1..3]),
                                      0,
                                      [0_u8; 256],
                    ).await.unwrap())
                }
                3 | 4 => {
                    let mut message = [0_u8; 256];
                    for (i, ch) in buf[2..].iter().enumerate() {
                        message[i] = *ch;
                    }
                    Some(Message::new(buf[0],
                                      0,
                                      buf[1],
                                      message,
                    ).await.unwrap())
                }
                _ => {
                    std::process::exit(1)
                }
            }
        } else {
            None
        }
    }
    async fn receive_and_expect3(stream: Arc<Mutex<TcpStream>>,
                                 expected_command: u8) -> Option<Self> {
        loop {
            let mut buf = [0_u8; 258];
            let _ = stream.lock().await.peek(&mut buf).await;
            if buf[0] == expected_command {
                let mut buf = [0_u8; 258];
                let _ = stream.lock().await.read_exact(&mut buf).await;
                match &buf[0] {
                    0 | 1 | 2 => {
                        return Some(Message::new(buf[0],
                                          NetworkEndian::read_u16(&buf[1..3]),
                                          0,
                                          [0_u8; 256],
                        ).await.unwrap())
                    }
                    3 | 4 => {
                        let mut message = [0_u8; 256];
                        for (i, ch) in buf[2..].iter().enumerate() {
                            message[i] = *ch;
                        }
                        return Some(Message::new(buf[0],
                                          0,
                                          buf[1],
                                          message,
                        ).await.unwrap())
                    }
                    _ => {
                        std::process::exit(1)
                    }
                }
            }
        }
    }
}

pub async fn interact_with_server(stream: Arc<Mutex<TcpStream>>,
                                  client_udp_port: u16) -> Result<()> {
    // 1. send hello
    println!("0");
    let hello: Message = Message::new(0, client_udp_port, 0, [0_u8; 256]).await?;
    println!("1");
    let _ = hello.send(stream.clone()).await;
    println!("sent hello");
    // 2. receive welcome
    let number_stations_temp: u16;
    let welcome: Message = Message::receive_and_expect(stream.clone(), 2).await?;
    println!("received welcome");
    if let MessageType::Data(number_stations) = welcome.message_type {
        // 3. save number_stations in local struct TODO
        number_stations_temp = number_stations;
        // 4. print required message
        println!("Welcome to Snowcast! The server has {} stations.",
                 &number_stations);
    } else {
        eprintln!("Received something other than the welcome message, exiting.");
        std::process::exit(1)
    }
    let number_stations: u16 = number_stations_temp;
    //let stream_clone = stream.clone();
    //thread::spawn(move || wait_for_announce(stream_clone));
    // 5. send set_station message in loop

    //let station_number = get_station_number2(number_stations);
    //println!("got station number");
    //let set_station: Message = Message::new(
    //    1, station_number.await, 0, [0_u8; 256]).await?;
    //let _ = set_station.send(stream.clone()).await;
    loop {
        println!("\nnew iteration of client loop\n");
        tokio::select! {
            station_number = get_station_number3(number_stations) => {
                println!("got station number");
                let set_station: Message = Message::new(
                    1, station_number, 0, [0_u8; 256]).await?;
                let _ = set_station.send(stream.clone()).await;
                let announcement = Message::receive_and_expect3(stream.clone(), 3).await.unwrap();
                println!("got an announcement in a different spot: {:?}", &announcement);
                // TODO sending set_station with the same value removes the 
                // value without keeping it set?
            }
            announcement = Message::receive_and_expect3(stream.clone(), 3) => {
                println!("got an announcement: {:?}", &announcement);
            }
        }
    }
    //loop {
    //    let station_number = loop {
    //        match get_station_number(number_stations) {
    //            Some(station_number) => break station_number,
    //            None => continue,
    //        }
    //    };
    //    println!("You selected station {}.", station_number);
    //    //let stream_clone = stream.clone();
    //    //let announcement_opt = thread::spawn(move || wait_for_announce(stream_clone));
    //    //let announcement = announcement_opt.join();
    //    //println!("announcement: {:?}", &announcement.unwrap().await);
    //    let set_station: Message = Message::new(
    //        1, station_number, 0, [0_u8; 256]).await?;
    //    tokio::select! {
    //        _stuff = wait_for_announce(stream.clone()) => {
    //            println!("announcement received!");
    //        }
    //        _things = set_station.send(stream.clone()) => {
    //            println!("send the things");
    //        }
    //    }
    //    //let _ = set_station.send(stream.clone()).await;
    //    //set_station(&stream, station_number)?;
    //}
}

async fn get_station_number(number_stations: u16) -> Option<u16> {
    println!("What station would you like to select? If you're done, \
              press \"q\" to exit.");
    let mut input = String::new();
    let _ = std::io::stdin().read_line(&mut input);
    let input: Vec<String> = input.split_whitespace().map(String::from).collect();
    //println!("{:?}", input);
    let station_number = if input.len() == 1 && input[0] == "q" {
        std::process::exit(1);
    } else if input.len() != 1 || input[0].parse::<u16>().is_err() || input[0].parse::<u16>().unwrap() > (number_stations - 1) {
        eprintln!("Pick a station from 0 to {} or quit with \"q\".", number_stations - 1);
        None// 'input
    } else {
        Some(input[0].parse::<u16>().unwrap())
    };
    station_number
}

async fn get_station_number2(number_stations: u16) -> u16 {
    println!("What station would you like to select? If you're done, \
              press \"q\" to exit.");
    let mut input = String::new();
    let _ = std::io::stdin().read_line(&mut input);
    let input: Vec<String> = input.split_whitespace().map(String::from).collect();
    //println!("{:?}", input);
    if input.len() == 1 && input[0] == "q" {
        std::process::exit(1);
    } else if input.len() >= 1 || input[0].parse::<u16>().is_err() || input[0].parse::<u16>().unwrap() > (number_stations - 1) {
        loop {
            eprintln!("Pick a station from 0 to {} or quit with \"q\".", number_stations - 1);
            let mut input = String::new();
            let _ = std::io::stdin().read_line(&mut input);
            let input: Vec<String> = input.split_whitespace().map(String::from).collect();
            if input.len() == 1 && input[0] == "q" {
                std::process::exit(1);
            } else if input.len() == 1 && input[0].parse::<u16>().unwrap() < (number_stations - 1) {
                return input[0].parse::<u16>().unwrap()
            }
        }
    } else if input.len() == 1 {
        return input[0].parse::<u16>().unwrap()
    } else {
        std::process::exit(1)
    }
}

async fn get_station_number3(number_stations: u16) -> u16 {
    println!("What station would you like to select? If you're done, \
              press \"q\" to exit.");
    let mut input = String::new();
    let _ = std::io::stdin().read_line(&mut input);
    let input: Vec<String> = input.split_whitespace().map(String::from).collect();
    //println!("{:?}", input);
    if input.len() == 1 && input[0] == "q" {
        std::process::exit(1);
    } else if input.len() == 1 && input[0].parse::<u16>().is_ok() && input[0].parse::<u16>().unwrap() <= (number_stations - 1) {
        return input[0].parse::<u16>().unwrap()
    } else {
        loop {
            eprintln!("Pick a station from 0 to {} or quit with \"q\".", number_stations - 1);
            let mut input = String::new();
            let _ = std::io::stdin().read_line(&mut input);
            let input: Vec<String> = input.split_whitespace().map(String::from).collect();
            if input.len() == 1 && input[0] == "q" {
                std::process::exit(1);
            } else if input.len() == 1 && input[0].parse::<u16>().is_ok() && input[0].parse::<u16>().unwrap() < (number_stations - 1) {
                return input[0].parse::<u16>().unwrap()
            }
        }
    }
}

//TODO do something with received announce now
async fn wait_for_announce(stream: Arc<Mutex<TcpStream>>) -> Option<Message> {
    let mut buf = [0_u8; 258];
    println!("something holds after this");
    let _ = stream.lock().await.peek(&mut buf).await;
    println!("{:?}", &buf);
    //if buf[0] == 3 {
    //    //stream.read().unwrap().read_exact(&mut buf.unwrap())?;
    //    return Some(Message::receive_and_expect(stream.clone(), 3).await.unwrap())
    //}
    thread::sleep(time::Duration::from_secs(1));
    None
}

async fn wait_for_announce2(stream: Arc<Mutex<TcpStream>>, tx: Sender<Option<Message>>) -> Result<()>/*Option<Message>*/ {
    loop {
        let mut buf = [0_u8; 258];
        let _ = stream.lock().await.peek(&mut buf);
        if buf[0] == 3 {
            //stream.lock().unwrap().read_exact(&mut buf.unwrap())?;
            //return Some(Message::receive_and_expect(stream.clone(), 3).unwrap())
            let _ = tx.send(Some(Message::receive_and_expect(stream.clone(), 3).await.unwrap()));
            return Ok(())
        }
    }
}
//fn wait_for_announce2(stream: Arc<Mutex<TcpStream>>) -> Option<

pub async fn handle_client(stream: Arc<Mutex<TcpStream>>,
                           song_path_vec: Vec<String>,
                           station_vec: Vec<Station>) -> Result<()> {
    // 1. receive hello
    let mut client_udp_port_temp: u16 = 65535;
    println!("waiting for hello");
    let hello: Message = Message::receive_and_expect(stream.clone(), 0).await?;
    if let MessageType::Data(udp_port) = hello.message_type {
        // 2. save client_udp_port in local structure TODO
        client_udp_port_temp = udp_port;
    }
    let client_udp_port: u16 = client_udp_port_temp;

    // 3. reply with the number of stations available in a welcome
    let welcome = Message::new(2, song_path_vec.len() as u16, 0, [0_u8; 256]).await?;
    //let stream_clone = stream.clone();
    let _ = welcome.send(stream.clone()).await;

    // 4. start a loop to wait for set station with client
    let mut prev_station_opt: Option<Station> = None;
    let mut prev_station_num_opt: Option<u16> = None;
    //let station_vec_lock = station_vec.lock().unwrap(); //can't do this bc
                                                          // it would always
                                                          // have the lock
    loop {
        tokio::select! {
            set_station = Message::receive_and_expect3(stream.clone(), 1) => {
                let station_opt: Option<Station>;
                if let MessageType::Data(station_number) = set_station.unwrap().message_type {
                    //let station_opt = Some(station_vec.get(station_number as usize).unwrap());
                    //if let Some(station) = station_opt {
                    //    station.udp_ports.write().await.push(client_udp_port);
                    //}
                    station_opt = Some(station_vec.get(station_number as usize).unwrap().clone());
                    if let Some(ref station) = station_opt {
                        if let Some(prev_station_num) = prev_station_num_opt {
                            if prev_station_num != station_number {
                                station.udp_ports.write().await.push(client_udp_port);
                            }
                        } else if let None = prev_station_num_opt {
                            station.udp_ports.write().await.push(client_udp_port);
                        }
                    }
                    if let Some(prev_station) = prev_station_opt {
                        if let Some(prev_station_num) = prev_station_num_opt {
                            if prev_station_num != station_number {
                                prev_station.udp_ports.write().await.retain(|&x| x != client_udp_port);
                            }
                        }
                    }
                    prev_station_opt = station_opt.clone();
                    prev_station_num_opt = Some(station_number.clone());

                } else {
                    eprintln!("something has really gone wrong");
                    std::process::exit(1)
                }
                let mut song_path_arr = [0_u8; 256];
                for (i, ch) in station_opt.clone()
                                          .unwrap()
                                          .song_path
                                          .as_bytes()
                                          .iter()
                                          .enumerate() {
                    song_path_arr[i] = *ch;
                }
                // 6. announce song playing on new station
                let announce = Message::new(3, 0, station_opt.unwrap().song_path.len() as u8, song_path_arr).await?;
                println!("\n\n\n\n\n\n\n\n\n\nsending announcement!");
                let _ = announce.send(stream.clone()).await;
            }
            _something_else = Message::receive_and_expect3(stream.clone(), 4) => {
                println!("got some error message!");
            }
        }
    }
    //loop {
    //    //let stream_clone = stream.clone();
    //    println!("just above set_station");
    //    //let set_station = Message::receive_and_expect(stream.clone(), 1).await?;
    //    let set_station = Message::receive_and_expect2(stream.clone(), 1).await.unwrap();
    //    //let station_vec_lock = station_vec.lock().unwrap();
    //    if let MessageType::Data(station_number) = set_station.message_type {
    //        // 5. receive a station number from client, and add udp port to a 
    //        // shareable struct that will be used in a udp_player loop
    //        //let station_vec_clone = station_vec.clone();
    //        // let station_vec_lock = station_vec.lock().unwrap();
    //        // let station_got = station_vec_lock.get(station_number as usize);
    //        //let station_vec_clone = station_vec.clone();
    //        let station_opt = Some(station_vec.get(station_number as usize).unwrap());
    //        println!("\n\n\nstuck ahead of stat\n\n\n");
    //        if let Some(station) = station_opt {
    //            println!("writing to udp");
    //            station.udp_ports.write().await.push(client_udp_port);
    //        }
    //        println!("\n\n\nstuck ahead of prev_stat\n\n\n");
    //        if let Some(prev_station) = prev_station_opt {
    //            println!("writing to udp");
    //            //if station_number != prev_station_number //TODO something like this??
    //            prev_station.udp_ports.write().await.retain(|&x| x != client_udp_port);
    //        }
    //        //TODO add functionality to drop stream if client disconnects
    //        prev_station_opt = station_opt.cloned();
    //        println!("{}, {:?}", station_opt.unwrap().song_path, station_opt.unwrap().udp_ports.read().await);
    //        //let announce = Message::receive_and_expect(stream.clone(), 3)?;
    //        let mut song_path_arr = [0_u8; 256];
    //        for (i, ch) in station_opt.unwrap()
    //                                  .song_path
    //                                  .as_bytes()
    //                                  .iter()
    //                                  .enumerate() {
    //            song_path_arr[i] = *ch;
    //        }
    //        // 6. announce song playing on new station
    //        let announce = Message::new(3, 0, station_opt.unwrap().song_path.len() as u8, song_path_arr).await?;
    //        println!("\n\n\n\n\n\n\n\n\n\nsending announcement!");
    //        let _ = announce.send(stream.clone()).await;
    //    }
    //}
}

pub async fn play_all_loops(server_name: Ipv4Addr,
                            server_udp_port: u16,
                            station_vec: Vec<Station>) -> Result<()> {
    for station in station_vec {
        let file = Arc::new(Mutex::new(File::open(&station.song_path).unwrap()));
        //let _ = play_song_loop(&file, &station.song_path, server_name, server_udp_port, &station.udp_ports);
        println!("spawning playing thread");
        //thread::spawn(move || play_song_loop(file, station.song_path, server_name, server_udp_port, station.udp_ports));
        tokio::spawn(async move {
            play_song_loop(file, station.song_path, server_name, server_udp_port, station.udp_ports).await
        });
    }
    Ok(())
}

async fn play_song_loop(file: Arc<Mutex<File>>,
                        song_path: String,
                        server_name: Ipv4Addr,
                        server_udp_port: u16,
                        client_udp_port_vec: Arc<RwLock<Vec<u16>>>) -> Result<()> {
    println!("thread spawned!");
    let time_gap = std::time::Duration::from_micros(62500);
    //let time_gap = std::time::Duration::from_secs(3);
    let file_len = File::open(&song_path).unwrap().seek(std::io::SeekFrom::End(0)).unwrap();
    println!("\n\n\nfile_len: {:?}\n\n\n", file_len);
    loop {
        let progress = file.clone().lock().await.stream_position().unwrap();
        let rel_pos = progress.clone() as f64 / file_len.clone() as f64;
        println!("rel_pos: {}", &rel_pos);
        println!("progress: {}", &progress);
        println!("file_len: {}", &file_len);
        let _ = play_song_chunk(file.clone(),
                                song_path.clone(),
                                server_name,
                                server_udp_port,
                                client_udp_port_vec.clone(),
        ).await;
        thread::sleep(time_gap);
    }
}

//async fn play_song2(file: File,
//                   song_path: String,
//                   server_name: Ipv4Addr,
//                   server_udp_port: u16,
//                   client_udp_port_vec: Arc<RwLock<Vec<u16>>>) -> Result<()> {
//
//    let client_udp_port_vec_unlock = client_udp_port_vec.read().await;
//    for udp in client_udp_port_vec_unlock.iter() {
//        let _ = play_song_chunk2(file, song_path.clone(), server_name.clone(), server_udp_port.clone(), *udp):
//    };
//    Ok(())
//}

//async fn play_song_chunk2(file: File,
//                          song_path: String,
//                          server_name: Ipv4Addr,
//                          server_udp_port: u16,
//                          client_udp_port: u16) -> Result<File> {
//    const BUF_LEN: u64 = 1024;
//    let mut song_buf = [0_u8; BUF_LEN as usize];
//    
//    song_buf = if current_pos + BUF_LEN > file_len {
//        let _ = file.lock().await.read_exact(&mut song_buf);
//        let _ = file.lock().await.seek(SeekFrom::Start(0));
//        song_buf
//    } else {
//        let _ = file.lock().await.read_exact(&mut song_buf);
//        song_buf
//    };
//}


async fn play_song_chunk(file: Arc<Mutex<File>>,
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
    let current_pos = file.lock().await.stream_position().unwrap();

    song_buf = if current_pos + BUF_LEN > file_len {
        let _ = file.lock().await.read_exact(&mut song_buf);
        let _ = file.lock().await.seek(SeekFrom::Start(0));
        song_buf
    } else {
        let _ = file.lock().await.read_exact(&mut song_buf);
        song_buf
    };
    println!("read file in {}: {:?}", &song_path, std::time::SystemTime::now());
    let socket: UdpSocket = UdpSocket::bind(format!("{}:{}", server_name, server_udp_port)).await?;
    //let client_udp_port
    //let client_udp_port_vec_read = 'read: loop {
    //    let client_udp_port_vec_read_opt = Some(client_udp_port_vec.read().unwrap());
    //    if let Some(client_udp_port_vec_read) = client_udp_port_vec_read_opt {
    //        break 'read client_udp_port_vec_read
    //        //break 'read
    //    }
    //};
    println!("client_port_vec in play_song_chunk{:?}",
        client_udp_port_vec.read().await);
    for udp in client_udp_port_vec.read().await.iter() {
        println!("{}", &udp);
        let _ = socket.connect(format!("{}:{}", server_name, udp)).await;
        let _ = socket.send(&song_buf).await;
    }
    Ok(())
}
