use core::slice::SlicePattern;
use std::net::{TcpStream, UdpSocket, Ipv4Addr};
use std::io::{Error, ErrorKind, Read, Result, Seek, Write};
use std::fs::File;
use std::string::FromUtf8Error;
use std::sync::{Mutex, Arc, RwLock};
use std::{thread, time};
//use crate::structs;
use byteorder::{ByteOrder, NetworkEndian};

//enum Message {
//    Hello (u8, u16),
//    SetStation (u8, u16),
//    Welcome (u8, u16),
//    Announce (u8, u8, Vec<u8>),
//    InvalidCommand (u8, u8, Vec<u8>),
//}
//
//impl Message {
//    fn send(&self, stream: Mutex<TcpStream>) -> Result<Vec<u8>> {
//        let vec = match self {
//            Message::Hello(command, udp_port) => {
//                let mut vec = vec!(*command);
//                vec.push(udp_port.to_be_bytes()[0]);
//                vec.push(udp_port.to_be_bytes()[1]);
//                vec
//            }
//            Message::SetStation(command, station_number) => {
//                let mut vec = vec!(*command);
//                vec.push(station_number.to_be_bytes()[0]);
//                vec.push(station_number.to_be_bytes()[1]);
//                vec
//            }
//            Message::Welcome(command, number_stations) => {
//                let mut vec = vec!(*command);
//                vec.push(number_stations.to_be_bytes()[0]);
//                vec.push(number_stations.to_be_bytes()[1]);
//                vec
//            }
//        };
//        stream.lock().unwrap().write_all(&vec);
//        stream.lock().unwrap().flush();
//    }
//}

enum Message {
    Short (u8, u16),
    Long (u8, u8, Vec<u8>),
}

impl Message {
    fn new(command: u8,
           data: u16,
           message_length: u8,
           message: Vec<u8>) -> Result<Self> {
        match &command {
            0 | 1 | 2 => {
                Ok(Message::Short(command, data))
            }
            3 | 4 => {
                Ok(Message::Long(command, message_length, message))
            }
            _ => {
                eprintln!("Received command other than 0-4, in Message::new()):
                          {}. Exiting now.", &command);
                std::process::exit(1)
            }
        }
    }
    fn send(&self, stream: Arc<Mutex<TcpStream>>) -> Result<()> {
        let buf: Vec<u8> = match self {
            Message::Short(command, data) => {
                //let mut vec = vec!(*command);
                //vec.push(data.to_be_bytes()[0]);
                //vec.push(data.to_be_bytes()[1]);
                let vec = vec!(*command,
                               data.to_be_bytes()[0],
                               data.to_be_bytes()[1]);
                vec
            }
            Message::Long(command, message_length, message) => {
                let mut vec = vec!(*command, *message_length);
                //vec.push(*message);
                for i in message {
                    vec.push(*i);
                }
                let vec: Vec<u8> = vec;
                vec
            }
        };
        stream.lock().unwrap().write_all(&buf);
        stream.lock().unwrap().flush();
        Ok(())
    }
    fn receive(stream: Arc<Mutex<TcpStream>>) -> Result<Self> {
        let mut buf = Vec::new();
        match stream.lock().unwrap().read_to_end(&mut buf) {
            Ok(_n_bytes) => {
                match &buf[0] {
                    0 | 1 | 2 => {
                        //Ok(Message::Short(buf[0],
                        //               NetworkEndian::read_u16(&buf[1..3])))
                        Message::new(buf[0],
                                     NetworkEndian::read_u16(&buf[1..3]),
                                     0,
                                     Vec::new())
                    }
                    3 | 4 => {
                        //Ok(Message::Long(buf[0],
                        //              buf[1],
                        //              buf[2..].to_vec()))
                        Message::new(buf[0],
                                     0,
                                     buf[1],
                                     buf[2..].to_vec())
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
            }

        }
    }
}

pub fn handle_client(stream: Arc<Mutex<TcpStream>>) -> Result<()> {
    // 1. receive hello

    match Message::receive(stream) {
        Ok(Message::Short(command, data)) => {
            if command == 0 {
                //TODO save udp_port in data to some struct?
            } else {
                eprintln!("Received a non-hello short message in handshake, /
                           exiting.");
                std::process::exit(1)
            }
        }
        Ok(Message::Long(command, message_length, message)) => {
            eprintln!("Received a non-short message in handshake, exiting.");
            eprintln!("{}, {}, {:?}", command, message_length, message);
            std::process::exit(1)
        }
        Err(error) => {
            eprintln!("Received an error while expecting hello: {}", error);
            std::process::exit(1)
        }
    }
    // 2. save client_udp_port in local structure



    // 3. reply with the number of stations available in a welcome

    let welcome = Message::new(2, song_path_vec_len, 0, Vec::new())?;
    welcome.send(stream);

    // 4. start a loop to wait for set station with client

    loop {
        Message::receive(stream)

    }

    // 5. receive a station number from client, and add udp port to a shareable
    //    struct that will be used in a udp_player loop
    // 6. 
    Ok(())
}
