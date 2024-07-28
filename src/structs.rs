use core::slice::SlicePattern;
use std::net::{TcpStream, UdpSocket, Ipv4Addr};
use std::io::{ErrorKind, Read, Result, Seek, Write};
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
    fn send(&self, stream: Mutex<TcpStream>) -> Self {
        let vec: Vec<u8> = match self {
            Message::Short(command, data) => {
                let mut vec = vec!(*command);
                vec.push(data.to_be_bytes()[0]);
                vec.push(data.to_be_bytes()[1]);
                vec
            }
            Message::Long(command, message_length, message) => {
                let mut vec = vec!(*command, *message_length);
                //vec.push(*message);
                for i in message {
                    vec.push(*i);
                vec
                }
            }
        };
    }
}

pub fn handle_client() -> Result<()> {
    // 1. receive hello

    receive_message()

    // 2. save client_udp_port in local structure
    // 3. reply with the number of stations available in a welcome
    // 4. start a loop to wait for set station with client
    // 5. receive a station number from client, and add udp port to a shareable
    //    struct that will be used in a udp_player loop
    // 6. 
}
