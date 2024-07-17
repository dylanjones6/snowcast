use std::collections::{HashMap, hash_map::Entry};
use std::net::{TcpStream, UdpSocket, Ipv4Addr};
use std::io::{ErrorKind, Read, Result, Seek, Write};
use std::fs::File;
//use std::sync::mpsc;
use std::sync::Mutex;
use std::{thread, time};
use byteorder::ReadBytesExt;


//use crate::structs;
use byteorder::{BigEndian, ByteOrder, NetworkEndian/*, ReadBytesExt, WriteBytesExt*/};


pub struct Hello {
    //direction: Send,
    command_type: u8, // should be == 0
    udp_port: u16,
}

pub struct SetStation {
    command_type: u8, // should be == 1 station_number: u16,
    station_number: u16,
}

#[derive(Debug)]
pub struct Welcome {
    reply_type: u8,
    number_stations: u16,
}

pub struct Announce <'a> {
    reply_type: u8,
    songname_size: u8,
    songname: &'a[u8],
}

pub struct InvalidCommand <'a> {
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
    ReplyAnnounceSC(Announce <'a>),
    ReplyInvalidCommandSC(InvalidCommand <'a>),
}


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
                            number_stations: second_u16
        })))}
        3 => {
            // Announce
            Ok(
                MessageSC::ReplyMessageSC(
                    ReplySC::ReplyAnnounceSC(
                        Announce {
                            reply_type: 3,
                            songname_size: data[1],
                            songname: &data[2..]
        })))}
        4 => {
            // InvalidCommand
            Ok(
                MessageSC::ReplyMessageSC(
                    ReplySC::ReplyInvalidCommandSC(
                        InvalidCommand {
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
                }
                SendSC::SendSetStationSC(set_station) => {
                    let mut data = [0 as u8; 512];
                    data[0] = set_station.command_type;
                    BigEndian::write_u16(&mut data[1..3], set_station.station_number);
                    return Ok(data.into())
                }
            }
        }
        MessageSC::ReplyMessageSC(reply) => {
            match reply {
                ReplySC::ReplyWelcomeSC(welcome) => {
                    let mut data = [0 as u8; 512];
                    data[0] = welcome.reply_type;
                    BigEndian::write_u16(&mut data[1..3], welcome.number_stations);
                    return Ok(data.into())
                }
                ReplySC::ReplyAnnounceSC(announce) => {
                    let mut data = [0 as u8; 512];
                    data[0] = announce.reply_type;
                    data[1] = announce.songname_size;
                    for i in 0..(announce.songname_size + 1) {
                        data[(i + 2) as usize] = announce.songname[i as usize]
                    }
                    return Ok(data.into())
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
                }
            }
        }
    }
}

pub fn initiate_handshake(mut stream: TcpStream, udp_port: &u16) -> Result<TcpStream> {

    let hello = MessageSC::SendMessageSC(SendSC::SendHelloSC(
                    Hello { command_type: 0, udp_port: *udp_port,}
    ));
    let data = *parse_enum_to_arr(hello).unwrap();
    println!("printing data sent from client in handshake: {:?}", &data);
    let _ = stream.write_all(&data);
    println!("Hello sent, awaiting response.");

    let mut data = [0 as u8; 3];
    let _n_bytes = stream.read_exact(&mut data)?;
    println!("data read by client following hello send: {:?}", &data);
    let message = if let Ok(
                            MessageSC::ReplyMessageSC(
                                ReplySC::ReplyWelcomeSC(
                                    welcome))) = parse_array_to_enum(&data) {
        welcome
    } else {
        panic!("Uh oh! Received something other than a welcome message.");
    };

    println!("Welcome to Snowcast! The server has {} stations.",
              message.number_stations);
    Ok(stream)

}

pub fn handle_client (mut stream: TcpStream, ip: &Ipv4Addr,
    file_vec: Vec<String>,
    //_tx: mpsc::Sender<HashMap<u16, Vec<u16>>>,
    //_rx: mpsc::Receiver<HashMap<u16, Vec<u16>>>,
    mut active_stations: Mutex<HashMap<u16, Vec<u16>>>) -> Result<()> {

    let mut data = [0 as u8; 3];
    let _ = stream.read_exact(&mut data)?;
    println!("data read by server at top of handle_client: {:?}", &data);
    let file_vec_clone = file_vec.clone();
    let number_stations: u16 = file_vec_clone.len().try_into().unwrap();

    let message = match parse_array_to_enum(&data) {
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
    println!("printing data from client: {:?}", &data);

    let welcome = MessageSC::ReplyMessageSC(ReplySC::ReplyWelcomeSC(
                    Welcome {
                        reply_type: 2,
                        number_stations, //TODO fn returns number_stations
    }));
    let data = *parse_enum_to_arr(welcome)?;
    println!("welcome sent from server: {:?}", &data);
    let _ = stream.write_all(&data);
    //let first_time = true;
    
    
    loop {
        let mut data = [0 as u8; 3];
        stream.read_exact(&mut data)?; //TODO NEED TO AWAIT RESULT HERE
        println!("data read by server anticipating set_station: {:?}", &data);
        match parse_array_to_enum(&mut data) {
            Ok(MessageSC::SendMessageSC(
                    SendSC::SendSetStationSC(set_station))) => {
                //UPDATE THE ACTIVE STATIONS

                // match active_stations.iter().find_map(|(key, &vec)| if vec.contains(&message.udp_port) { Some(key) } else { None }) {
                //     Some(old_key) => {
                //         match active_stations.entry(old_key) {
                //             Entry::Vacant(entry) => {

                //             }
                //             Entry::Occupied(entry) => {

                //             }

                //         }

                //     }
                //     None => {
                //         match active_stations.entry(set_station.station_number) {
                //             Entry::Vacant(entry) => { entry.insert(vec![message.udp_port]); },
                //             Entry::Occupied(mut entry) => { entry.get_mut().push(message.udp_port); },
                //         }
                //     }

                // }
                let key = match active_stations.lock().unwrap().iter().find_map(|(key, &ref vec)| if vec.contains(&message.udp_port) { Some(key) } else { None }) {
                    Some(old_key) => old_key.clone(),
                    None => set_station.station_number.clone(),
                };
                match active_stations.lock().unwrap().entry(key) {
                    Entry::Vacant(entry) => {
                        entry.insert(vec![message.udp_port]);
                    }
                    Entry::Occupied(mut entry) => {
                        entry.get_mut().push(message.udp_port);
                    }
                }
                let _ = broadcast_song(&file_vec[set_station.station_number as usize], ip, message.udp_port);
                

                //set_station.station_number
                //set_station
            }
            Ok(MessageSC::SendMessageSC(SendSC::SendHelloSC(_))) => {
                panic!("Received a Hello message, that's not good.");
            }
            Ok(MessageSC::ReplyMessageSC(_)) => {
                panic!("Received a reply message, that's not good.");
            }
            Err(error) => {
                panic!("Error parsing array: {}", error);
            }
        };
    }
    //NetworkEndian::write_
    //} else if data[0] == 1 {
    //    // SetStation branch, receive and deal with station accordingly and send out
    //    // announcement immediately
    //    //let announce = Reply::Announce { reply_type: 3, songname_size: 0, songname: &[0] }; //TODO need to implement song name and size stuff
    //    Ok(())
    //} else {
    //    panic!("Message sent against protocol!");
    //    //TODO implement InvalidCommand
    //}
}

pub fn broadcast_song(song_path: &str,/* songname: String,*/ server_name: &Ipv4Addr, udp_port: u16) -> Result<()> {
    
    // BIG TODO: maybe flip this somehow so the looping is always happening 
    // and writing to stream is done on a per thread basis? or split into play
    // song and broadcast functions, play_song() plays a file chunk by chunk
    // and broadcast() sends out the info to the given port

    let full_ip = format!("{}:{}", server_name, udp_port);
    let socket = UdpSocket::bind(full_ip).expect("Couldn't bind to address.");
    // 16384 bytes/second = 1024 bytes * 16 /sec  // MUST be < 1500 bytes/sec
    // 1024 bytes every .0625 sec
    //
    // 16384 bytes/second = 64 bits or 8 bytes * 2048 / sec
    // u64 every 0.00048828125
    //
    //let mut buf = [0 as u8; 1024];
    let mut buf = [0 as u8; 8];
    let mut file = File::open(song_path)?;
    //let time_gap = time::Duration::from_micros(62500);
    let time_gap_num = 488280;
    let time_gap = time::Duration::from_nanos(time_gap_num);
    let smaller_time_gap = time::Duration::from_nanos(time_gap_num/8);
    loop { //song loop
        let _ = file.rewind();
        'within_song: loop{
            match file.read_u64::<NetworkEndian>() { //read methods advance cursor so we
                                              //don't need to move positions through
                                              //the file
                Ok(data) => {
                    NetworkEndian::write_u64(&mut buf, data);
                    let _ = &mut socket.send(&buf);
                    thread::sleep(time_gap);
                }
                Err(error) => match error.kind() {
                    ErrorKind::UnexpectedEof => {
                        // let mut vec = Vec::new();
                        // let _ = file.read_to_end(&mut vec);
                        // let _ = socket.send(&vec);
                        // let _ = file.rewind(); // this is maybe the issue if song doesn't loop
                        // break 'within_song;
                        for _ in 0..8 {
                            match file.read_u8() {
                                Ok(data) => {
                                    let _ = &mut socket.send(&[data]);
                                    thread::sleep(smaller_time_gap);
                                }
                                Err(error) => match error.kind() {
                                    ErrorKind::UnexpectedEof => {
                                        break 'within_song
                                    }
                                    _ => {
                                        panic!("Ran into a different error while completing the song: {}", error);
                                    }
                                }
                            }
                        };
                        //let mut buf = [0 as u8; 1024];
                        //let arr: [u8; 1024] = vec.as_slice().try_into().unwrap();
                        //vec.try_into().unwrap_or_else(|vec: Vec<T> | panic!("Issue creating array"));
                    }
                    _ => {
                        panic!("Error while reading song file: {}", song_path);
                    }
                }
            }
        }
    }
}

pub fn set_station(mut stream: TcpStream, station_number: u16) -> Result<TcpStream> {
    let set_station = MessageSC::SendMessageSC(
        SendSC::SendSetStationSC(
            SetStation {
                command_type: 1,
                station_number,
    }));
    let data = *parse_enum_to_arr(set_station)?;
    println!("data sent from set_station: {:?}", &data);
    match stream.write_all(&data) {
        Ok(_) => {
            return Ok(stream)
        }
        Err(error) => {
            panic!("Error caused while set_station wrote to stream: {}",
                error);
        }
    }
}
