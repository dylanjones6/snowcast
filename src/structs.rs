use std::net::{TcpStream, UdpSocket, Ipv4Addr};
use std::io::{ErrorKind, Read, Result, Seek, Write};
use std::fs::File;
use std::string::FromUtf8Error;
use std::sync::{Mutex, Arc, RwLock};
use std::{thread, time};
//use crate::structs;
use byteorder::{ByteOrder, NetworkEndian};


pub struct Hello {
    //direction: Send,
    pub command_type: u8, // should be == 0
    pub udp_port: u16,
}

impl Clone for Hello {
    fn clone(&self) -> Self {
        Self {
            command_type: self.command_type.clone(),
            udp_port: self.udp_port.clone(),
        }
    }
}

pub struct SetStation {
    pub command_type: u8, // should be == 1 station_number: u16,
    pub station_number: u16,
}

pub struct Welcome {
    pub reply_type: u8,
    pub number_stations: u16,
}

pub struct Announce {
    pub reply_type: u8,
    pub songname_size: u8,
    pub songname: [u8; 256],
}

pub struct InvalidCommand {
    pub reply_type: u8,
    pub reply_string_size: u8,
    pub reply_string: [u8; 256],
}

//gotta rewrite receive to use array or something not pattern matching
#[derive(Debug)]
pub struct Station {
    pub song_path: String,
    pub udp_ports: Arc<Mutex<Vec<u16>>>,
    //pub port_ind_running: Vec<bool>,
}

impl Station {
    pub fn new(song_path: String, udp_ports: Arc<Mutex<Vec<u16>>>) -> Self {
        Self {
            song_path, 
            udp_ports,
        }
    }
    pub fn add_udp_port(&self, new_udp_port: u16) {
        let _ = &self.udp_ports.lock().unwrap().push(new_udp_port);
    }
}

impl Clone for Station {
    fn clone(&self) -> Self {
        Self {
            song_path: self.song_path.clone(),
            udp_ports: self.udp_ports.clone(),  // Clone fields that require it
            // Clone other fields similarly
        }
    }
}


struct ServerData {
    server_name: Ipv4Addr,
    tcp_port: u16,
    udp_port: Arc<RwLock<Vec<u16>>>,
    //number_stations: u16,
    stream: Mutex<TcpStream>,
    station_number: RwLock<u16>,
    song_path_vec: Vec<String>,
    open_file_vec: Arc<Mutex<Vec<File>>>,
}

impl ServerData {
    fn get_number_stations(&self) -> Result<u16> {
        Ok(self.song_path_vec.len() as u16)
    }
    fn get_songname_size(&self) -> Result<u8> {
        Ok(self.song_path_vec[self.station_number.read().unwrap().clone() as usize].len() as u8)
    }
    fn get_songname(&self) -> Result<[u8; 256]> {
        let mut song = [0_u8; 256];
        for (i, ch) in self.song_path_vec[self.station_number.read().unwrap().clone() as usize]
                                         .bytes()
                                         .enumerate() {
            song[i] = ch;
        }
        Ok(song)
    }
    fn get_songname_string(&self) -> Result<String> {
        let songname_arr = self.get_songname().unwrap();
        let songname_vec = songname_arr.to_vec();
        Ok(String::from_utf8(songname_vec).unwrap())
    }
}

struct ClientData <'a>{
    server_name: Ipv4Addr,
    tcp_port: u16,
    udp_port: u16,
    number_stations: u16,
    station_number: RwLock<u16>,
    songname_size: u8,
    songname: &'a [u8],
}

struct DataContainer <'a> {
    server_name: Ipv4Addr, //input for client at startup; static
    tcp_port: u16, //input for client at startup; static
    udp_port: u16, //input for client at startup, comm. w/ hello; static (w/in thread)
    number_stations: u16, //input for server at startup, comm. w/ welcome; static
    station_number: RwLock<u16>, //input for client at set_station prompt, 
                                 //comm. w/ set_station
    current_songname_size: u8,
    current_songname: &'a [u8],
    //song_path_vec: Vec<&'a str>, //input for server, never comm.
}

pub enum MessageSC {
    SendMessageSC(SendSC),
    ReplyMessageSC(ReplySC),
}

// using ...SC naming scheme bc of collision with Send trait
pub enum SendSC {
    SendHelloSC(Hello),
    SendSetStationSC(SetStation),
}

pub enum ReplySC {
    ReplyWelcomeSC(Welcome),
    ReplyAnnounceSC(Announce),
    ReplyInvalidCommandSC(InvalidCommand),
}

pub fn parse_array_to_enum(data: [u8; 258]) -> Result<MessageSC>  {
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
            Ok( MessageSC::SendMessageSC(
                    SendSC::SendSetStationSC(
                        SetStation {
                            command_type: 1,
                            station_number: second_u16 
        })))}
        2 => {
            // Welcome
            Ok(
                MessageSC::ReplyMessageSC(
                    ReplySC::ReplyWelcomeSC(
                        Welcome {
                            reply_type: 2,
                            number_stations: second_u16
        })))}
        3 => {
            // Announce
            let mut songname_data = [0_u8; 256];
            songname_data.copy_from_slice(&data[2..]);
            Ok(
                MessageSC::ReplyMessageSC(
                    ReplySC::ReplyAnnounceSC(
                        Announce {
                            reply_type: 3,
                            songname_size: data[1],
                            songname: songname_data,
        })))}
        4 => {
            // InvalidCommand
            let mut reply_string_data = [0_u8; 256];
            reply_string_data.copy_from_slice(&data[2..]);
            Ok(
                MessageSC::ReplyMessageSC(
                    ReplySC::ReplyInvalidCommandSC(
                        InvalidCommand {
                            reply_type: 4,
                            reply_string_size: data[1],
                            reply_string: reply_string_data,
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


pub fn parse_enum_to_arr(message: MessageSC) -> Result<Box<[u8; 258]>> {
    match message {
        MessageSC::SendMessageSC(send) => {
            //send::<SendHello, SendSetStation>.command_type; // use traits instead
            match send {
                SendSC::SendHelloSC(hello) => {
                    // //let mut data = [hello.command_type, hello.udp_port.to_be_bytes().iter()];
                    let mut data = [0_u8; 258];
                    data[0] = hello.command_type;
                    NetworkEndian::write_u16(&mut data[1..3], hello.udp_port);
                    //data[1..3].copy_from_slice(&hello.udp_port.to_be_bytes());
                    Ok(data.into())
                }
                SendSC::SendSetStationSC(set_station) => {
                    let mut data = [0_u8; 258];
                    data[0] = set_station.command_type;
                    NetworkEndian::write_u16(&mut data[1..3], set_station.station_number);
                    Ok(data.into())
                }
            }
        }
        MessageSC::ReplyMessageSC(reply) => {
            match reply {
                ReplySC::ReplyWelcomeSC(welcome) => {
                    let mut data = [0_u8; 258];
                    data[0] = welcome.reply_type;
                    NetworkEndian::write_u16(&mut data[1..3], welcome.number_stations);
                    Ok(data.into())
                }
                ReplySC::ReplyAnnounceSC(announce) => {
                    let mut data = [0_u8; 258];
                    data[0] = announce.reply_type;
                    data[1] = announce.songname_size;
                    for i in 0..(announce.songname_size + 1) {
                        data[(i + 2) as usize] = announce.songname[i as usize]
                    }
                    Ok(data.into())
                }
                ReplySC::ReplyInvalidCommandSC(invalid_command) => {
                    let mut data = [0_u8; 258];
                    data[0] = invalid_command.reply_type;
                    data[1] = invalid_command.reply_string_size;
                    //BigEndian::write_u16(&mut data[2..(announce.songname_size + 1)], announce.songname);
                    for i in 0..(invalid_command.reply_string_size + 1) {
                        data[(i + 2) as usize] = invalid_command.reply_string[i as usize]
                    }
                    Ok(data.into())
                }
            }
        }
    }
}

pub fn initiate_handshake(stream: &Mutex<TcpStream>, udp_port: &u16) -> Result<Welcome> {
    let _ = send_hello(stream, udp_port);
    receive_welcome(stream)
}

pub fn handle_client (stream: Mutex<TcpStream>,
                      server_name: Ipv4Addr,
                      tcp_port: u16,
                      udp_port_vec: Arc<RwLock<Vec<u16>>>,
                      song_path_vec: Vec<String>,
                      /*station_vec: Vec<Station>*/) -> Result<()> {

    ////let hello: Hello = receive_hello(&stream)?;
    //let hello: Hello = if let MessageSC::SendMessageSC(
    //    SendSC::SendHelloSC(hello) = receive_message(&stream, hello_check)
    //)
    let hello = if let Ok(MessageSC::SendMessageSC(
            SendSC::SendHelloSC(
                hello))) = receive_message(&stream, 0) {
        //received_hello();
        hello
    } else {
        eprintln!("No hello returned from received_message, exiting.");
        std::process::exit(1)
    };

    //let file_vec_clone = file_vec.clone();
    //let number_stations: u16 = file_vec.len().try_into().unwrap();
    let number_stations: u16 = song_path_vec.len().try_into().unwrap();

    ////let _ = send_welcome(&stream, number_stations);
    // send welcome message in response
    let _ = send_message(&stream, 2, number_stations, 0, [0; 256]);
    //let mut data = [0_u8; 258];
    //
    let open_file_vec: Arc<Mutex<Vec<File>>> = Arc::new(Mutex::new(Vec::new()));
    for song_path in song_path_vec {
        open_file_vec.lock().unwrap().push(File::open(song_path)?);
    }

    let server_data = ServerData {
        server_name,
        tcp_port,
        udp_port: hello.udp_port, //need to make this like an arc
        stream,
        station_number: RwLock::new(65535),
        song_path_vec,
        open_file_vec,
    };

    loop {
        //let station_vec_clone = station_vec.clone();
        ////let _ = receive_set_station(&stream, &hello, station_vec.clone());
        if let Ok(MessageSC::SendMessageSC(
                SendSC::SendSetStationSC(
                    set_station))) = receive_message(&stream, 1) {
            //let _ = received_set_station(&stream, &set_station, &hello, &station_vec);
            let _ = received_set_station2(server_data, set_station);
        };
    }
}

//struct ServerData {
//    server_name: Ipv4Addr,
//    tcp_port: u16,
//    udp_port: u16,
//    //number_stations: u16,
//    station_number: RwLock<u16>,
//    song_path_vec: Vec<String>,
//    open_file_vec: Arc<Mutex<Vec<File>>>,
//}

// fn send_hello(stream: &Mutex<TcpStream>, udp_port: &u16) -> Result<()>{
//     let hello = MessageSC::SendMessageSC(SendSC::SendHelloSC(
//                     Hello { command_type: 0, udp_port: *udp_port,}
//     ));
//     let data = *parse_enum_to_arr(hello).unwrap();
//     println!("printing data sent from client in handshake: {:?}", &data);
//     let _ = stream.lock().unwrap().write_all(&data);
//     let _ = stream.lock().unwrap().flush();
// 
//     println!("Hello sent, awaiting response.");
// 
//     Ok(())
// }

//TODO CREATE GENERALIZED SEND_MESSAGE THING (ALSO RECEIVE)
pub fn send_message (stream: &Mutex<TcpStream>,
                     message_type: u8,
                     long_arg: u16,
                     short_arg: u8,
                     string_arg: [u8; 256]) -> Result<()> {
    let (message, message_name) = match message_type {
        0 => {
            (MessageSC::SendMessageSC(
                SendSC::SendHelloSC(
                    Hello {
                        command_type: message_type,
                        udp_port: long_arg,
            })), String::from("Hello"))
        }
        1 => {
            (MessageSC::SendMessageSC(
                SendSC::SendSetStationSC(
                    SetStation {
                        command_type: message_type,
                        station_number: long_arg,
            })), String::from("SetStation"))
        }
        2 => {
            (MessageSC::ReplyMessageSC(
                ReplySC::ReplyWelcomeSC(
                    Welcome {
                        reply_type: message_type,
                        number_stations: long_arg,
            })), String::from("Welcome"))
        }
        3 => {
            (MessageSC::ReplyMessageSC(
                ReplySC::ReplyAnnounceSC(
                    Announce {
                        reply_type: message_type,
                        songname_size: short_arg,
                        songname: string_arg,
            })), String::from("Announce"))
        }
        4 => {
            (MessageSC::ReplyMessageSC(
                ReplySC::ReplyInvalidCommandSC(
                    InvalidCommand {
                        reply_type: message_type,
                        reply_string_size: short_arg,
                        reply_string: string_arg,
            })), String::from("InvalidCommand"))
        }
        _ => {
            eprintln!("Received invalid command, in send_command. Exiting \
                       program now.");
            std::process::exit(1)
        }
    };
    let data = *parse_enum_to_arr(message)?;
    match stream.lock().unwrap().write_all(&data) {
        Ok(_) => {
            //let _ = stream.lock().unwrap().flush();
            //println!("extra test");
            Ok(())
        }
        Err(error) => {
            eprintln!("Error caused while {} wrote to stream: {}",
                message_name, error);
            std::process::exit(1)
        }
    }
}

// fn receive_hello(stream: &Mutex<TcpStream>) -> Result<Hello> {
//     let mut data = [0_u8; 258];
//     let _ = stream.lock().unwrap().read_exact(&mut data)?;
//     println!("data read by server at top of handle_client: {:?}", &data);
// 
//     let hello = match parse_array_to_enum(&data) {
//         Ok(MessageSC::SendMessageSC(
//             SendSC::SendHelloSC(hello))) => {
//             hello
//         }
//         Ok(_) => {
//             panic!("Wrong kind of message!");
//         }
//         Err(error) => {
//             panic!("Error parsing array: {}", error);
//         }
//     };
//     Ok(hello)
// }


//pub fn receive_message2(stream: &Mutex<TcpStream>,
//                        expected_command: u8) -> Result<MessageSC> {
//    let mut data = [0_u8; 258];
//    let _ = stream.lock().unwrap().read_exact(&mut data)?;
//    match &data[0] {
//        0 => {
//            Ok()
//        }
//    }
//    //match parse_parse_array_to_enum(data) {
//    //    Ok(message) => {
//    //        match message {
//    //            MessageSC::SendMessageSC(
//    //                SendSC::SendHelloSC(
//    //                    hello) => {
//    //                    hell
//    //                }
//    //            )
//    //        }
//    //    }
//
//    //}
//
//}

///```
///let stream = Mutex::new(TcpStream::connect("127.0.0.1:7878").unwrap());
///let mut write_data = [0_u8; 256];
///write_data[0] = 0;
///write_data[1] = 12;
///write_data[2] = 34;
///stream.lock().unwrap().write_all(&write_data);
///let hello = MessageSC::SendMessageSC(
///                SendSC::SendHelloSC(
///                    Hello {
///                        command_type: 0,
///                        udp_port: 42069,
///                    }
///                )
///            );
///let hello_out = receive_message(&stream, hello).unwrap();
///let command_test = if let MessageSC::SendMessageSC(
///                             SendSC::SendHelloSC(
///                                 hello)) = hello_out {
///                                     hello.command_type
///                   } else {
///                    std::process::exit(1)
///                   };
///assert_eq!(0, command_test);
///```
pub fn receive_message(stream: &Mutex<TcpStream>,
                       expected_command: u8) -> Result<MessageSC> {
    let mut data = [0_u8; 258];
    stream.lock().unwrap().read_exact(&mut data)?;
    //println!("data read by server at top of handle_client: {:?}", &data);
    match parse_array_to_enum(data) {
        Ok(message) => {
            //let message = message;
            match (&message, &expected_command) {
                (MessageSC::SendMessageSC(
                    SendSC::SendHelloSC(
                        _)), 0) => {
                    Ok(message)
                }
                (MessageSC::SendMessageSC(
                    SendSC::SendSetStationSC(
                        _)), 1) => {
                    Ok(message)
                }
                (MessageSC::ReplyMessageSC(
                    ReplySC::ReplyWelcomeSC(
                        _)), 2) => {
                    Ok(message)
                }
                (MessageSC::ReplyMessageSC(
                    ReplySC::ReplyAnnounceSC(
                        _)), 3) => {
                    Ok(message)
                }
                (MessageSC::ReplyMessageSC(
                    ReplySC::ReplyInvalidCommandSC(
                        _)), 4) => {
                    Ok(message)
                }
                _ => {
                    eprintln!("Read and expected files do not match!");
                    std::process::exit(1)
                }
            }
        }
        Err(error) => Err(error),
    }
    // match parse_array_to_enum(data) {
    //     Ok(message) => {
    //         if message == expected_message {
    //             Ok(message)
    //         } else {
    //             eprintln!("Found something other than what was expected!");
    //             std::process::exit(1)
    //         }
    //     }
    //     Err(error) => Err(error),
    // }
    //match parse_array_to_enum(&data) {
    //    Ok(message) => {
    //        let message::<'a>: MessageSC = message;
    //        match expected_message {
    //            message => {
    //                return Ok(message)
    //            }
    //        }
    //    }
    //    Err(error) => {
    //        eprintln!("{}", error);
    //        std::process::exit(1)
    //    }
    //    Err(_) => panic!("Something else!"),
    //}

    //let message = match parse_array_to_enum(&data) {
    //    //Ok(MessageSC::SendMessageSC(
    //    //    SendSC::SendHelloSC(hello))) => {
    //    //    hello
    //    //}
    //    Ok(message) => {
    //        let message;
    //        match &message {
    //            MessageSC::SendMessageSC(
    //                SendSC::SendHelloSC()
    //            )
    //        }
    //    }
    //    Ok(_) => {
    //        panic!("Wrong kind of message!");
    //    }
    //    Err(error) => {
    //        panic!("Error parsing array: {}", error);
    //    }
    //};
    //Ok(hello)

}

// pub fn send_message (stream: &Mutex<TcpStream>,
//                      message: MessageSC) -> Result<()> {
//     //println!("data sent from set_station: {:?}", &data);
//     let message_type = match &message {
//         MessageSC::SendMessageSC(send) => {
//             match send {
//                 SendSC::SendHelloSC(_) => {
//                     String::from("Hello")
//                 }
//                 SendSC::SendSetStationSC(_) => {
//                     String::from("SetStation")
//                 }
//             }
//         }
//         MessageSC::ReplyMessageSC(reply) => {
//             match reply {
//                 ReplySC::ReplyWelcomeSC(_) => {
//                     String::from("Welcome")
//                 }
//                 ReplySC::ReplyAnnounceSC(_) => {
//                     String::from("Announce")
//                 }
//                 ReplySC::ReplyInvalidCommandSC(_) => {
//                     String::from("InvalidCommand")
//                 }
//             }
//         }
//     };
//     let data = *parse_enum_to_arr(message)?;
//     match stream.lock().unwrap().write_all(&data) {
//         Ok(_) => {
//             //let _ = stream.lock().unwrap().flush();
//             //println!("extra test");
//             return Ok(())
//         }
//         Err(error) => {
//             panic!("Error caused while {} wrote to stream: {}",
//                 message_type, error);
//         }
//     }
// }

pub fn set_station(stream: &Mutex<TcpStream>, station_number: u16) -> Result<()> {
    let set_station = MessageSC::SendMessageSC(
        SendSC::SendSetStationSC(
            SetStation {
                command_type: 1,
                station_number,
    }));
    let data = *parse_enum_to_arr(set_station)?;
    //println!("data sent from set_station: {:?}", &data);
    match stream.lock().unwrap().write_all(&data) {
        Ok(_) => {
            //let _ = stream.lock().unwrap().flush();
            //println!("extra test");
            Ok(())
        }
        Err(error) => {
            panic!("Error caused while set_station wrote to stream: {}",
                error);
        }
    }
}

fn receive_hello(stream: &Mutex<TcpStream>) -> Result<Hello> {
    let mut data = [0_u8; 258];
    stream.lock().unwrap().read_exact(&mut data)?;
    println!("data read by server at top of handle_client: {:?}", &data);

    let hello = match parse_array_to_enum(data) {
        Ok(MessageSC::SendMessageSC(
            SendSC::SendHelloSC(hello))) => {
            hello
        }
        Ok(_) => {
            panic!("Wrong kind of message!");
        }
        Err(error) => {
            panic!("Error parsing array: {}", error);
        }
    };
    Ok(hello)
}

fn send_welcome(stream: &Mutex<TcpStream>, number_stations: u16) -> Result<()> {
    let welcome = MessageSC::ReplyMessageSC(ReplySC::ReplyWelcomeSC(
                    Welcome {
                        reply_type: 2,
                        number_stations, //TODO fn returns number_stations
    }));
    let data = *parse_enum_to_arr(welcome)?;
    println!("welcome sent from server: {:?}", &data);
    let _ = stream.lock().unwrap().write_all(&data);
    let _ = stream.lock().unwrap().flush();

    Ok(())
}

fn send_hello(stream: &Mutex<TcpStream>, udp_port: &u16) -> Result<()>{
    let hello = MessageSC::SendMessageSC(SendSC::SendHelloSC(
                    Hello { command_type: 0, udp_port: *udp_port,}
    ));
    let data = *parse_enum_to_arr(hello).unwrap();
    println!("printing data sent from client in handshake: {:?}", &data);
    let _ = stream.lock().unwrap().write_all(&data);
    let _ = stream.lock().unwrap().flush();

    println!("Hello sent, awaiting response.");

    Ok(())
}

fn receive_welcome(stream: &Mutex<TcpStream>) -> Result<Welcome> {
    let mut data = [0_u8; 258];
    stream.lock().unwrap().read_exact(&mut data)?;
    println!("data read by client following hello send: {:?}", &data);
    let welcome = if let Ok(
                            MessageSC::ReplyMessageSC(
                                ReplySC::ReplyWelcomeSC(
                                    welcome))) = parse_array_to_enum(data) {
        welcome
    } else {
        panic!("Uh oh! Received something other than a welcome message.");
    };

    println!("Welcome to Snowcast! The server has {} stations.",
              welcome.number_stations);
    Ok(welcome)
}

fn received_set_station2(server_data: &ServerData,
                         set_station: &SetStation) {

    
}

fn received_set_station(stream: &Mutex<TcpStream>,
                        set_station: &SetStation,
                        hello: &Hello,
                        station_vec: &Vec<Station>) -> Result<()> {
    if let Some(station) = station_vec.get(set_station.station_number as usize) {
        println!("id be amazed if this printed");
        station.udp_ports.lock().unwrap().push(hello.udp_port);
    }
    let mut song = [0_u8; 256];
    //let _ = station_vec[set_station.station_number as usize].song_path.bytes().enumerate().map(|(i, letter)| song[i] = letter);
    for (i, ch) in station_vec[set_station.station_number as usize]
                              .song_path
                              .bytes()
                              .enumerate() {
        if i >= 256 {
            break
        }
        song[i] = ch;
    }
    //println!("song: {:?}", &song);
    let song_len: u8 = station_vec[set_station.station_number as usize].song_path.len() as u8;
    let _ = send_message(stream, 3, 0, song_len, song);
    Ok(())
}

//fn receive_set_station(stream: &Mutex<TcpStream>,
//                       hello: &Hello,
//                       station_vec: Vec<Station>,
//                       /*ip: &Ipv4Addr*/) -> Result<()> {
//    let mut data = [0_u8; 258];
//
//    stream.lock().unwrap().read_exact(&mut data)?;
//    println!("data read by server anticipating set_station: {:?}", &data);
//    match parse_array_to_enum(data) {
//        Ok(MessageSC::SendMessageSC(
//                SendSC::SendSetStationSC(set_station))) => {
//            println!("hey you're close!");
//            if let Some(station) = station_vec.get(set_station.station_number as usize) {
//                println!("id be amazed if this printed");
//                station.udp_ports.lock().unwrap().push(hello.udp_port);
//            }
//            println!("added port to station_vec");
//            Ok(())
//        }
//        Ok(MessageSC::SendMessageSC(SendSC::SendHelloSC(_))) => {
//            panic!("Received a Hello message, that's not good.");
//        }
//        Ok(MessageSC::ReplyMessageSC(_)) => {
//            panic!("Received a reply message, that's not good.");
//        }
//        Err(error) => {
//            panic!("Error parsing array: {}", error);
//        }
//    }
//}

//fn send_announce(stream: &Mutex<TcpStream>,
//                 songname_length: u8,
//                 songname: [u8; 256]) -> Result<()> {
//
//
//}

// pub fn play_loop(station_vec: Vec<Station>,
//               server_name: Ipv4Addr,
//               open_file_vec: Arc<Mutex<Vec<File>>>) -> Result<()> {
//     loop {
//         let _ = play_all_songs_chunk(station_vec.clone(), server_name, open_file_vec.clone());
//     }
// }


fn play_song_loop(server_data: &ServerData) -> Result<()> {
    loop {
        let _ = play_song_chunk(server_data);
    }
}

fn play_song_chunk(server_data: &ServerData) -> Result<()> {
    let time_gap = time::Duration::from_micros(62500);
    let mut song_buf = [0_u8; 1024];
    let station_int = server_data.station_number.read().unwrap().clone() as usize;
    let mut open_file_vec = server_data.open_file_vec.lock().unwrap();
    let current_file: &mut File = open_file_vec.get_mut(station_int).unwrap();//[server_data.station_number.read().unwrap().clone() as usize];
    match current_file.read_exact(&mut song_buf) {
        Ok(_) => {},
        Err(error) => match error.kind() {
            ErrorKind::UnexpectedEof => {
                let _ = current_file.rewind();
            }
            _ => {
                eprintln!("Issue reading file {}", server_data.get_songname_string().unwrap());
                return Err(ErrorKind::InvalidData.into())
            }
        }
    }
    for udp_port in server_data.udp_port.read().unwrap().iter() {
        let socket: UdpSocket = UdpSocket::bind(
            format!("{}:{}", &server_data.server_name, &server_data.tcp_port))?;
        let _ = socket.connect(format!("{}:{}", &server_data.server_name, &udp_port))?;
        let _ = socket.send(&song_buf);
    }
    thread::sleep(time_gap);
    Ok(())
}

fn play_all_songs_chunk(station_vec: Vec<Station>,
                        server_name: Ipv4Addr,
                        open_file_vec: Arc<Mutex<Vec<File>>>) -> Result<()> {
    let time_gap = time::Duration::from_micros(62500);
    //let smaller_time_gap = time::Duration::from_millis(5);
    for (i, song) in station_vec.iter().enumerate() {
        //let file = File::open(song.song_path)?;
        let mut song_buf = [0_u8; 1024];
        let mut current_file = open_file_vec.lock().unwrap();
        let current_file = current_file.get_mut(i).unwrap();
        match current_file.read_exact(&mut song_buf) {
            Ok(_) => {},
            Err(error) => match error.kind() {
                ErrorKind::UnexpectedEof => {
                    let _ = current_file.rewind();
                }
                _ => {
                    panic!("some other error");
                }
            }

        };
        for udp_port in song.udp_ports.lock().unwrap().iter() {
            let socket: UdpSocket = UdpSocket::bind("127.0.0.1:7878")?;
            let _ = socket.connect(format!("{}:{}", &server_name, &udp_port));
            let _ = socket.send(&song_buf);
            //thread::sleep(smaller_time_gap);
        }
    }
    thread::sleep(time_gap);
    Ok(())
}

//
//
//      server:                                         client:
//  1. wait for hello <-----------------------> 1. send hello w/ udp_port
//  2. receive hello, spawn thread, store <-|   2. wait for welcome
//     udp_port, send welcome w/ # stats.       3. receive welcome, store + 
//  3. wait for set_station                        print # stations
//  4. receive set_station, store               4. send set_station w/ stat. #
//     station_number, announce song in         5. 
//     response
//  5. wait for new set_station
//  6. quit and send quit command (5)
