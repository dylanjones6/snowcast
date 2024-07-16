use std::net::{TcpStream, Ipv4Addr};
use std::io::{/*Error, */Read, Result, Write};

//use crate::structs;
use byteorder::{BigEndian, ByteOrder, NetworkEndian, /*ReadBytesExt, */WriteBytesExt};


pub struct Hello {
    //direction: Send,
    command_type: u8, // should be == 0
    udp_port: u16,
}

pub struct SetStation {
    command_type: u8, // should be == 1
    station_number: u16,
}

pub struct Welcome {
    reply_type: u8,
    num_stations: u16,
}

// pub struct Announce <'a> { 
//     reply_type: u8,
//     songname_size: u8,
//     songname: &'a [u8],
// }

pub struct Announce {
    reply_type: u8,
    songname_size: u8,
    songname: Vec<u8>,
}

pub struct AnnounceArr <'a> {
    reply_type: u8,
    songname_size: u8,
    songname: &'a[u8],
}

// pub struct InvalidCommand <'a> {
//     reply_type: u8,
//     reply_string_size: u8,
//     reply_string: &'a [u8],
// }

pub struct InvalidCommand {
    reply_type: u8,
    reply_string_size: u8,
    reply_string: Vec<u8>,
}

pub struct InvalidCommandArr <'a> {
    reply_type: u8,
    reply_string_size: u8,
    reply_string: &'a[u8],
}

pub enum MessageSC <'a> {
    SendMessageSC(SendSC),
    ReplyMessageSC(ReplySC <'a>),
}

// using ...SC naming scheme bc of collision with Send trait
pub enum SendSC {
    SendHelloSC(Hello),
    SendSetStationSC(SetStation),
}

pub enum ReplySC <'a> {
    ReplyWelcomeSC(Welcome),
    ReplyAnnounceSC(AnnounceArr <'a>),
    ReplyInvalidCommandSC(InvalidCommandArr <'a>),
}

// pub fn parse_vec_to_enum <'a> (data: Vec<u8>) -> Result<MessageSC<'a>> {
//     let second_u16: u16 = NetworkEndian::read_u16(&data[1..3]); //used for first
//                                                                 //3 cases
//     match &data[0] {
//         0 => {
//             // Hello
//             Ok(
//                 MessageSC::SendMessageSC(
//                     SendSC::SendHelloSC(
//                         Hello {
//                             command_type: 0,
//                             udp_port: second_u16,
//         })))}
//         1 => {
//             // SetStation
//             Ok(
//                 MessageSC::SendMessageSC(
//                     SendSC::SendSetStationSC(
//                         SetStation {
//                             command_type: 0,
//                             station_number: second_u16 
//         })))}
//         2 => {
//             // Welcome
//             Ok(
//                 MessageSC::ReplyMessageSC(
//                     ReplySC::ReplyWelcomeSC(
//                         Welcome {
//                             reply_type: 2,
//                             num_stations: second_u16
//         })))}
//         3 => {
//             // Announce
//             let songname: Vec<u8> = data[2..].to_vec();
//             Ok(
//                 MessageSC::ReplyMessageSC(
//                     ReplySC::ReplyAnnounceSC(
//                         Announce {
//                             reply_type: 3,
//                             songname_size: data[1],
//                             songname,
//         })))}
//         4 => {
//             // InvalidCommand
//             let reply_string: Vec<u8> = data[2..].to_vec();
//             Ok(
//                 MessageSC::ReplyMessageSC(
//                     ReplySC::ReplyInvalidCommandSC(
//                         InvalidCommand {
//                             reply_type: 4,
//                             reply_string_size: data[1],
//                             reply_string,
//         })))}
//         _ => {
//             //Err(())
//             panic!("Data does not match Snowcast protocol!"); //TODO better error handling, return
//                                                               //error enum or something 
//                 //eprintln!("Data does not match Snowcast protocol!")
//                 //eprintln!("Data read: {}", &data)
//         }
//     }
// }


pub fn parse_array_to_enum (data: &[u8]) -> Result<MessageSC>  {
    let second_u16: u16 = NetworkEndian::read_u16(&data[1..3]); //used for first
                                                                //3 cases
    match &data[0] {
        0 => {
            // Hello
            Ok(
                MessageSC::SendMessageSC(
                    SendSC::SendHelloSC(
                        Hello {
                            command_type: 0,
                            udp_port: second_u16 
        })))}
        1 => {
            // SetStation
            Ok(
                MessageSC::SendMessageSC(
                    SendSC::SendSetStationSC(
                        SetStation {
                            command_type: 0,
                            station_number: second_u16 
        })))}
        2 => {
            // Welcome
            Ok(
                MessageSC::ReplyMessageSC(
                    ReplySC::ReplyWelcomeSC(
                        Welcome {
                            reply_type: 2,
                            num_stations: second_u16
        })))}
        3 => {
            // Announce
            Ok(
                MessageSC::ReplyMessageSC(
                    ReplySC::ReplyAnnounceSC(
                        AnnounceArr {
                            reply_type: 3,
                            songname_size: data[1],
                            songname: &data[2..]
        })))}
        4 => {
            // InvalidCommand
            Ok(
                MessageSC::ReplyMessageSC(
                    ReplySC::ReplyInvalidCommandSC(
                        InvalidCommandArr {
                            reply_type: 4,
                            reply_string_size: data[1],
                            reply_string: &data[2..]
        })))}
        _ => {
            //Err(())
            panic!("Data does not match Snowcast protocol!"); //TODO better error handling, return
                                                              //error enum or something 
                //eprintln!("Data does not match Snowcast protocol!")
                //eprintln!("Data read: {}", &data)
        }
    }
}

// pub fn parse_enum_to_data(message: MessageSC) -> Result<Vec<u8>> {
//     match message {
//         MessageSC::SendMessageSC(send) => {
//             //send::<SendHello, SendSetStation>.command_type; // use traits instead
//             match send {
//                 SendSC::SendHelloSC(hello) => {
//                     // //let mut data = [hello.command_type, hello.udp_port.to_be_bytes().iter()];
//                     // let mut data = [0 as u8; 3];
//                     // data[0] = hello.command_type;
//                     // BigEndian::write_u16(&mut data[1..3], hello.udp_port);
//                     // //data[1..3].copy_from_slice(&hello.udp_port.to_be_bytes());
//                     // return Ok(&data)
//                     let mut vec = Vec::new();
//                     vec.push(hello.command_type); //don't specify enum bc it's only one byte
//                     vec.write_u16::<NetworkEndian>(hello.udp_port);
//                     Ok(vec)
//                 }
//                 SendSC::SendSetStationSC(set_station) => {
//                     // let mut data = [0 as u8; 3];
//                     // data[0] = set_station.command_type;
//                     // BigEndian::write_u16(&mut data[1..3], set_station.station_number);
//                     // return Ok(&data)
//                     let mut vec = Vec::new();
//                     vec.push(set_station.command_type); //don't specify enum bc it's only one byte
//                     vec.write_u16::<NetworkEndian>(set_station.station_number);
//                     Ok(vec)
//                 }
//             }
//         }
//         MessageSC::ReplyMessageSC(reply) => {
//             match reply {
//                 ReplySC::ReplyWelcomeSC(welcome) => {
//                     // let mut data = [0 as u8; 3];
//                     // data[0] = welcome.reply_type;
//                     // BigEndian::write_u16(&mut data[1..3], welcome.num_stations);
//                     // return Ok(&data)
//                     let mut vec = Vec::new();
//                     vec.push(welcome.reply_type); //don't specify enum bc it's only one byte
//                     vec.write_u16::<NetworkEndian>(welcome.num_stations);
//                     Ok(vec)
//                 }
//                 ReplySC::ReplyAnnounceSC(announce) => {
//                     // let mut data = [0 as u8; 512];
//                     // data[0] = announce.reply_type;
//                     // data[1] = announce.songname_size;
//                     // BigEndian::write_u16(&data[2..], announce.songname);
//                     let mut vec = Vec::new();
//                     vec.push(announce.reply_type); //don't specify enum bc it's only one byte
//                     vec.push(announce.songname_size);
//                     for i in announce.songname { // TODO use iterator here instead
//                         vec.push(i)
//                     }
//                     //vec.push(announce.songname.into_iter());
//                     Ok(vec)
//                 }
//                 ReplySC::ReplyInvalidCommandSC(invalid_command) => {
//                     let mut vec = Vec::new();
//                     vec.push(invalid_command.reply_type); //don't specify enum bc it's only one byte
//                     vec.push(invalid_command.reply_string_size);
//                     for i in invalid_command.reply_string { // TODO use iterator here instead
//                         vec.push(i)
//                     }
//                     //vec.push(announce.songname.into_iter());
//                     Ok(vec)
//                 }
//             }
//         }
//     }
// }

pub fn parse_enum_to_arr <'a> (message: MessageSC) -> Result<Box<[u8; 512]>> {
    match message {
        MessageSC::SendMessageSC(send) => {
            //send::<SendHello, SendSetStation>.command_type; // use traits instead
            match send {
                SendSC::SendHelloSC(hello) => {
                    // //let mut data = [hello.command_type, hello.udp_port.to_be_bytes().iter()];
                    let mut data = [0 as u8; 512];
                    data[0] = hello.command_type;
                    NetworkEndian::write_u16(&mut data[1..3], hello.udp_port);
                    //data[1..3].copy_from_slice(&hello.udp_port.to_be_bytes());
                    Ok(data.into())
                    // let mut vec = Vec::new();
                    // vec.push(hello.command_type); //don't specify enum bc it's only one byte
                    // vec.write_u16::<NetworkEndian>(hello.udp_port);
                    // Ok(vec)
                }
                SendSC::SendSetStationSC(set_station) => {
                    let mut data = [0 as u8; 512];
                    data[0] = set_station.command_type;
                    BigEndian::write_u16(&mut data[1..3], set_station.station_number);
                    return Ok(data.into())
                    // let mut vec = Vec::new();
                    // vec.push(set_station.command_type); //don't specify enum bc it's only one byte
                    // vec.write_u16::<NetworkEndian>(set_station.station_number);
                    // Ok(vec)
                }
            }
        }
        MessageSC::ReplyMessageSC(reply) => {
            match reply {
                ReplySC::ReplyWelcomeSC(welcome) => {
                    let mut data = [0 as u8; 512];
                    data[0] = welcome.reply_type;
                    BigEndian::write_u16(&mut data[1..3], welcome.num_stations);
                    return Ok(data.into())
                    // let mut vec = Vec::new();
                    // vec.push(welcome.reply_type); //don't specify enum bc it's only one byte
                    // vec.write_u16::<NetworkEndian>(welcome.num_stations);
                    // Ok(vec)
                }
                ReplySC::ReplyAnnounceSC(announce) => {
                    let mut data = [0 as u8; 512];
                    data[0] = announce.reply_type;
                    data[1] = announce.songname_size;
                    //BigEndian::write_u16(&mut data[2..(announce.songname_size + 1)], announce.songname);
                    for i in 0..(announce.songname_size + 1) {
                        data[(i + 2) as usize] = announce.songname[i as usize]
                    }
                    return Ok(data.into())
                    // let mut vec = Vec::new();
                    // vec.push(announce.reply_type); //don't specify enum bc it's only one byte
                    // vec.push(announce.songname_size);
                    // for i in announce.songname { // TODO use iterator here instead
                    //     vec.push(i)
                    // }
                    // //vec.push(announce.songname.into_iter());
                    // Ok(vec)
                }
                ReplySC::ReplyInvalidCommandSC(invalid_command) => {
                    let mut data = [0 as u8; 512];
                    data[0] = invalid_command.reply_type;
                    data[1] = invalid_command.reply_string_size;
                    //BigEndian::write_u16(&mut data[2..(announce.songname_size + 1)], announce.songname);
                    for i in 0..(invalid_command.reply_string_size + 1) {
                        data[(i + 2) as usize] = invalid_command.reply_string[i as usize]
                    }
                    return Ok(data.into())
                    // let mut vec = Vec::new();
                    // vec.push(invalid_command.reply_type); //don't specify enum bc it's only one byte
                    // vec.push(invalid_command.reply_string_size);
                    // for i in invalid_command.reply_string { // TODO use iterator here instead
                    //     vec.push(i)
                    // }
                    // //vec.push(announce.songname.into_iter());
                    // Ok(vec)
                }
            }
        }
    }
}
            //  if let Reply::ReplyWelcome(welcome) = greeting {
            //      let mut welcome_vec = vec![welcome.reply_type];
            //      let mut welcome_vec2 = welcome.num_stations.to_be_bytes().to_vec();
            //      welcome_vec.append(&mut welcome_vec2);
            //      println!("coming from within enum!");
            //      println!("reply_type: {}", &welcome.reply_type);
            //      println!("num_stations: {}", &welcome.num_stations);
            //      println!("welcome_vec: {:?}", &welcome_vec);
            //      stream.write_all(&welcome_vec[..])?;
            

pub fn read_data (mut stream: TcpStream) -> Result<Box<[u8; 512]>> {
    let mut data = [0 as u8; 512];
    stream.read_exact(&mut data)?;
    Ok(data.into())
}

//pointless hmmmmm
pub fn send_data(mut stream: TcpStream, message: &[u8]) {
    let _ = stream.write_all(message);
}

//let something = MessageSC::SendMessageSC(SendSC::SendHelloSC(Hello ))

pub fn initiate_handshake(ip: &Ipv4Addr, server_port: &u16, udp_port: &u16) {
    let full_address = format!("{}:{}", ip, server_port);
    println!("{}", &full_address);
    println!("test");

    match TcpStream::connect(&full_address) {
        Ok(mut stream) => {
            println!("Connected to server at {}", &full_address);
            //let hello = MessageSC::SendMessageSC(SendSC::SendHelloSC(Hello { command_type: 0, udp_port: *udp_port }));
            //let something = MessageSC::SendMessageSC(structs::SendSC::SendHelloSC( Hello { command_type: 0, })));a
            //
            //  if let Reply::ReplyWelcome(welcome) = greeting {
            //      let mut welcome_vec = vec![welcome.reply_type];
            //      let mut welcome_vec2 = welcome.num_stations.to_be_bytes().to_vec();
            //      welcome_vec.append(&mut welcome_vec2);
            //      println!("coming from within enum!");
            //      println!("reply_type: {}", &welcome.reply_type);
            //      println!("num_stations: {}", &welcome.num_stations);
            //      println!("welcome_vec: {:?}", &welcome_vec);
            //      stream.write_all(&welcome_vec[..])?;
            //  }
            // if let MessageSC::SendMessageSC(SendSC::SendHelloSC(hello)) = hello {
            //     hello
            // } else {
            //     panic!("hello isn't hello");
            // };
            // let greeting = MessageSC::ReplyMessageSC(ReplySC::ReplyWelcomeSC(
            //                 Welcome {
            //                     reply_type: 2,
            //                     num_stations: 0 //TODO fn returns num_stations
            // }));
            // let data = *parse_enum_to_arr(greeting).unwrap();
            // let _ = stream.write_all(&data);

            let hello = MessageSC::SendMessageSC(SendSC::SendHelloSC(
                            Hello { command_type: 0, udp_port: *udp_port,}
            ));
            let data = *parse_enum_to_arr(hello).unwrap();
            println!("printing data sent from client: {:?}", &data);
            let _ = stream.write_all(&data);


            // let mut message_vec = vec![0 as u8];
            // let mut message_vec2 = udp_port.to_be_bytes().to_vec();
            // message_vec.append(&mut message_vec2);

            // stream.write_all(&message_vec).unwrap();
            println!("Hello sent, awaiting response.");

            //let mut data = Vec::new();
            let mut buf = [0 as u8; 3];
            //println!("data before read: {:?}", &buf);
            match stream.read_exact(&mut buf) {
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

pub fn handle_client(mut stream: TcpStream, _file_vec: Vec<String>) -> Result<()> {
    let mut data = [0 as u8; 3];
    //let mut data = vec![0; 3];
    match stream.read_exact(&mut data) {
        Ok(_) => {
            // if size != 3 { std::process::exit(1) };
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
                //let greeting2 = MessageSC::ReplyMessageSC(ReplySC::ReplyWelcomeSC(()))
                let greeting = MessageSC::ReplyMessageSC(ReplySC::ReplyWelcomeSC(
                                Welcome {
                                    reply_type: 2,
                                    num_stations: 0 //TODO fn returns num_stations
                }));
                let data = *parse_enum_to_arr(greeting).unwrap();
                println!("printing data from client: {:?}", &data);
                let _ = stream.write_all(&data);
                // if let MessageSC::ReplyMessageSC(ReplySC::ReplyWelcomeSC(welcome)) = greeting {
                //     let welcome_vec = parse_enum_to_data(welcome);
                //     // let mut welcome_vec = vec![welcome.reply_type];
                //     // let mut welcome_vec2 = welcome.num_stations.to_be_bytes().to_vec();
                //     // welcome_vec.append(&mut welcome_vec2);
                //     // println!("coming from within enum!");
                //     // println!("reply_type: {}", &welcome.reply_type);
                //     // println!("num_stations: {}", &welcome.num_stations);
                //     // println!("welcome_vec: {:?}", &welcome_vec);
                //     stream.write_all(&welcome_vec[..])?;
                // }
                
                loop {
                    let mut buf = [0 as u8; 3];
                    match stream.read(&mut buf) {
                        Ok(_size) => {

                        }
                        Err(_error) => {

                        }
                    };
                }
                //NetworkEndian::write_
            } else if data[0] == 1 {
                // SetStation branch, receive and deal with station accordingly and send out
                // announcement immediately
                //let announce = Reply::Announce { reply_type: 3, songname_size: 0, songname: &[0] }; //TODO need to implement song name and size stuff
                Ok(())
            } else {
                panic!("Message sent against protocol!");
                //TODO implement InvalidCommand
            }
        }
        Err(error) => {
            panic!("Error reading data stream: {error}");
        }
    }
}
