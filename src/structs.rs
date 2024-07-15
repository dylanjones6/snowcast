use std::net::{TcpStream, Ipv4Addr};
use std::io::{Error, Read, Result, Write};

use crate::structs;
use byteorder::{BigEndian, ByteOrder, NetworkEndian, ReadBytesExt, WriteBytesExt};



// pub enum Send {
//     Hello { command_type: u8, udp_port: u16 }, // command_type == 0
//     SetStation { command_type: u8, station_number: u16 }, // command_type == 1
// }
// 
// //pub enum Message {
// //    Hello()
// //}
// 
// 
// pub enum Reply <'a> {
//     Welcome { reply_type: u8, num_stations: u16 }, // reply_type == 2
//     Announce { reply_type: u8, songname_size: u8, songname: &'a[u8]}, // reply_type == 3
//     InvalidCommand { reply_type: u8, reply_string_size: u8, reply_string: &'a[u8]}, // reply_type == 4
// }
// 
// pub trait Marshall {
//     fn marshallize(&self) -> Vec<u8>;
// }


// enum Send {
//     Hello,
//     SetStation,
// }


pub struct Hello <'a> {
    //direction: Send,
    command_type: u8, // should be == 0
    udp_port: &'a u16,
}

impl <'a> Hello <'a> {
    pub fn create(udp_port: &'a u16) -> Self {
        Self {
            command_type: 0,
            udp_port,
        }
    }
}

pub trait Marshall {
    fn marshallize(&self) -> Vec<u8>;
}


// impl Marshall for Hello {
//     fn marshallize(&self) -> Vec<u8> {
//         
//     }
// }



// impl Default for Hello {
//     fn default() -> Hello {
//         Hello {
//             command_type: 0,
//             udp_port: 0,
//         }
//     }
// }


pub struct SetStation <'a> {
    command_type: u8, // should be == 1
    station_number: &'a u16,
}

impl <'a> SetStation <'a> {
    fn create(station_number: &'a u16) -> Self {
        Self {
            command_type: 1,
            station_number,
        }
    }
}

// impl Default for SetStation {
//     fn default() -> SetStation {
//         SetStation {
//             command_type: 1,
//             station_number: 0,
//         }
//     }
// }



pub struct Welcome <'a> {
    reply_type: u8,
    num_stations: &'a u16,
}

impl <'a> Welcome <'a> {
    pub fn create(num_stations: &'a u16) -> Self {
        Welcome {
            reply_type: 2,
            num_stations,
        }
    }
}


pub struct Announce <'a> {
    reply_type: u8,
    songname_size: &'a u8,
    songname: &'a[u8],
}

impl <'a> Announce <'a> {
    pub fn create(songname_size: &'a u8, songname: &'a[u8]) -> Self {
        Self {
            reply_type: 3,
            songname_size,
            songname,
        }
    }

}

// impl <'a> Default for Announce <'a> {
//     fn default() -> Announce <'a> {
//         Announce {
//             reply_type: 3,
//             songname_size: 0,
//             songname: &[0],
//         }
//     }
// }


pub struct InvalidCommand <'a> {
    reply_type: u8,
    reply_string_size: &'a u8,
    reply_string: &'a[u8],
}
impl <'a> InvalidCommand <'a> {
    pub fn create(reply_string_size: &'a u8, reply_string: &'a [u8]) -> Self {
        Self {
            reply_type: 4,
            reply_string_size,
            reply_string,
        }
    }
}

// impl <'a> Default for InvalidCommand <'a> {
//     fn default() -> InvalidCommand <'a> {
//         InvalidCommand {
//             reply_type: 4,
//             reply_string_size: 0,
//             reply_string: &[0],
//         }
//     }
// }

pub enum Message <'a> {
    SendMessage(Send <'a>),
    ReplyMessage(Reply <'a>), 
}

pub enum Send <'a> {
    SendHello(Hello <'a>),
    SendSetStation(SetStation <'a>),
}

pub enum Reply <'a> {
    ReplyWelcome(Welcome <'a>),
    ReplyAnnounce(Announce <'a>),
    ReplyInvalidCommand(InvalidCommand <'a>),
}

pub fn parse_to_enum <'a> (data: &[u8]) -> Result<Message>  {
    let mut second_u16: u16 = NetworkEndian::read_u16(&data[1..3]); //used for first
                                                                //3 cases
    match &data[0] {
        0 => {
            // Hello
            Ok(
                Message::SendMessage(
                    Send::SendHello(
                        Hello {
                            command_type: 0,
                            udp_port: second_u16 
                        }
                    )
                )
            )
        }
        1 => {
            // SetStation
            Ok(
                Message::SendMessage(
                    Send::SendSetStation(
                        SetStation {
                            command_type: 0,
                            station_number: second_u16 
                        }
                    )
                )
            )
        }
        2 => {
            // Welcome
            Ok(
                Message::ReplyMessage(
                    Reply::ReplyWelcome(
                        Welcome {
                            reply_type: 2,
                            num_stations: second_u16
                        }
                    )
                )
            )
        }
        3 => {
            // Announce
            Ok(
                Message::ReplyMessage(
                    Reply::ReplyAnnounce(
                        Announce {
                            reply_type: 3,
                            songname_size: &data[1],
                            songname: &data[2..]
                        }
                    )
                )
            )
        }
        4 => {
            // InvalidCommand
            Ok(
                Message::ReplyMessage(
                    Reply::ReplyInvalidCommand(
                        InvalidCommand {
                            reply_type: 4,
                            reply_string_size: &data[1],
                            reply_string: &data[2..]
                        }
                    )
                )
            )
        }
        _ => {
            //Err(())
            panic!("Data does not match Snowcast protocol!"); //TODO better error handling, return
                                                              //error enum or something 
                //eprintln!("Data does not match Snowcast protocol!")
                //eprintln!("Data read: {}", &data)
        }
    }
}

//let something = Message::SendMessage(Send::SendHello(Hello ))

pub fn initiate_handshake(ip: &Ipv4Addr, server_port: &u16, udp_port: &u16) {
    let full_address = format!("{}:{}", ip, server_port);
    println!("{}", &full_address);
    println!("test");

    match TcpStream::connect(&full_address) {
        Ok(mut stream) => {
            println!("Connected to server at {}", &full_address);
            let hello = Send::SendHello(Hello { command_type: 0, udp_port });
            //let something = Message::SendMessage(structs::Send::SendHello( Hello { command_type: 0, })));
            if let Send::SendHello(hello) = hello {
                hello
            } else {

            };
            let mut message_vec = vec!(0 as u8);
            let mut message_vec2 = udp_port.to_be_bytes().to_vec();
            message_vec.append(&mut message_vec2);

            stream.write_all(&message_vec).unwrap();
            println!("Hello sent, awaiting response.");

            //let mut data = Vec::new();
            let mut buf = [0 as u8; 3];
            //println!("data before read: {:?}", &buf);
            match stream.read(&mut buf) {
                Ok(_size) => {
                    //println!("data after read: {:?}", &buf);
                    if buf[0] == 2 {
                        println!("We received the welcome message!");
                    } else {
                        panic!("Received reply code: {}. Terminating program.", buf[0]);
                    }
                }
                Err(error) => eprintln!("Error reading server response: {}", error),
            }
        }
        Err(error) => {
            eprintln!("There was an issue connecting to {}.", full_address);
            eprintln!("Error: {}", error);
            panic!("Terminating program.");
        }
    }

}

pub fn handle_client(mut stream: TcpStream, file_vec: Vec<String>) -> Result<()> {
    //let mut data = [0 as u8; 512];
    let mut data = vec![0; 3];
    match stream.read(&mut data) {
        Ok(size) => {
            if size != 3 { std::process::exit(1)};
            if data[0] == 0 {
                //let welcome = Reply::Welcome { reply_type: 2, num_stations: 0 }; //TODO need to add station counting
                /*
                let num_stations = 0;
                let welcome = Welcome::create(&num_stations);
                let mut welcome_vec = vec![welcome.reply_type];
                let mut welcome_vec2 = welcome.num_stations.to_be_bytes().to_vec();
                welcome_vec.append(&mut welcome_vec2);
                println!("reply_type: {}", &welcome.reply_type);
                println!("num_stations: {}", &welcome.num_stations);
                println!("welcome_vec: {:?}", &welcome_vec);
                stream.write_all(&welcome_vec[..])?;
                */
                let greeting = Reply::ReplyWelcome(
                                                   Welcome {
                                                       reply_type: 2,
                                                       num_stations: &0
                                                   });
                if let Reply::ReplyWelcome(welcome) = greeting {
                    let mut welcome_vec = vec![welcome.reply_type];
                    let mut welcome_vec2 = welcome.num_stations.to_be_bytes().to_vec();
                    welcome_vec.append(&mut welcome_vec2);
                    println!("coming from within enum!");
                    println!("reply_type: {}", &welcome.reply_type);
                    println!("num_stations: {}", &welcome.num_stations);
                    println!("welcome_vec: {:?}", &welcome_vec);
                    stream.write_all(&welcome_vec[..])?;
                }
                
                loop {
                    let mut buf = [0 as u8; 3];
                    match stream.read(&mut buf) {
                        Ok(size) => {

                        }
                        Err(error) => {

                        }
                    };
                }
                //NetworkEndian::write_
            } else if data[0] == 1 {
                //let announce = Reply::Announce { reply_type: 3, songname_size: 0, songname: &[0] }; //TODO need to implement song name and size stuff
                Ok(())
            } else {
                panic!("Message sent against protocol!");
            }
        }
        Err(error) => {
            panic!("Error reading data stream: {error}");
        }
    }
}
