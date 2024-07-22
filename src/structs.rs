use std::collections::{HashMap, hash_map::Entry};
use std::net::{TcpStream, UdpSocket, Ipv4Addr};
use std::io::{ErrorKind, Read, Result, Seek, Write};
use std::fs::File;
//use std::sync::mpsc;
use std::sync::{Mutex, Arc};
use std::{thread, time};
//use crate::structs;
use byteorder::{BigEndian, ByteOrder, NetworkEndian, ReadBytesExt, WriteBytesExt};


pub struct Hello {
    //direction: Send,
    pub command_type: u8, // should be == 0
    pub udp_port: u16,
}

pub struct SetStation {
    pub command_type: u8, // should be == 1 station_number: u16,
    pub station_number: u16,
}

#[derive(Debug)]
pub struct Welcome {
    pub reply_type: u8,
    pub number_stations: u16,
}

pub struct Announce <'a> {
    pub reply_type: u8,
    pub songname_size: u8,
    pub songname: &'a[u8],
}

pub struct InvalidCommand <'a> {
    pub reply_type: u8,
    pub reply_string_size: u8,
    pub reply_string: &'a[u8],
}

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
        &self.udp_ports.lock().unwrap().push(new_udp_port);
    }
}

// struct Connection {
//     udp_ports: Arc<Mutex<Vec<u16>>>,
//     sockets: Arc<Mutex<Vec<UdpSocket>>>,
// }

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
            Ok( MessageSC::SendMessageSC( SendSC::SendSetStationSC(
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
                    NetworkEndian::write_u16(&mut data[1..3], set_station.station_number);
                    return Ok(data.into())
                }
            }
        }
        MessageSC::ReplyMessageSC(reply) => {
            match reply {
                ReplySC::ReplyWelcomeSC(welcome) => {
                    let mut data = [0 as u8; 512];
                    data[0] = welcome.reply_type;
                    NetworkEndian::write_u16(&mut data[1..3], welcome.number_stations);
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

pub fn initiate_handshake(stream: &Mutex<TcpStream>, udp_port: &u16) -> Result<Welcome> {
    let _ = send_hello(stream, udp_port);
    receive_welcome(stream)
}

pub fn handle_client (stream: Mutex<TcpStream>,
    server_name: Ipv4Addr,
    file_vec: Vec<String>,
    //_tx: mpsc::Sender<HashMap<u16, Vec<u16>>>,
    //_rx: mpsc::Receiver<HashMap<u16, Vec<u16>>>,
    station_vec: &mut Vec<Station>) -> Result<()> {

    let hello: Hello = receive_hello(&stream)?;

    //let file_vec_clone = file_vec.clone();
    let number_stations: u16 = file_vec.len().try_into().unwrap();

    let _ = send_welcome(&stream, number_stations);

    //let mut data = [0 as u8; 512];

    loop {
        receive_set_station(&stream, &hello, station_vec);
    }


    //data = loop {
    //    //let stream = stream.lock().unwrap();
    //    //thread::sleep(time::Duration::from_millis(1500));
    //    println!("waiting");
    //    let _ = stream.lock().unwrap().read_exact(&mut data)?;
    //    println!("data read in loop: {:?}", &data);
    //    if data[0] == 1 { break data }
    //};
    //let mut first_time = true;
    //loop {
    //}
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


pub fn set_station(stream: &Mutex<TcpStream>, station_number: u16) -> Result<()> {
    let set_station = MessageSC::SendMessageSC(
        SendSC::SendSetStationSC(
            SetStation {
                command_type: 1,
                station_number,
    }));
    let data = *parse_enum_to_arr(set_station)?;
    println!("data sent from set_station: {:?}", &data);
    match stream.lock().unwrap().write_all(&data) {
        Ok(_) => {
            let _ = stream.lock().unwrap().flush();
            return Ok(())
        }
        Err(error) => {
            panic!("Error caused while set_station wrote to stream: {}",
                error);
        }
    }
}

fn receive_hello(stream: &Mutex<TcpStream>) -> Result<Hello> {
    let mut data = [0 as u8; 512];
    let _ = stream.lock().unwrap().read_exact(&mut data)?;
    println!("data read by server at top of handle_client: {:?}", &data);

    let hello = match parse_array_to_enum(&data) {
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
    stream.lock().unwrap().flush();

    Ok(())
}

fn send_hello(stream: &Mutex<TcpStream>, udp_port: &u16) -> Result<()>{
    let hello = MessageSC::SendMessageSC(SendSC::SendHelloSC(
                    Hello { command_type: 0, udp_port: *udp_port,}
    ));
    let data = *parse_enum_to_arr(hello).unwrap();
    println!("printing data sent from client in handshake: {:?}", &data);
    let _ = stream.lock().unwrap().write_all(&data);
    stream.lock().unwrap().flush();

    println!("Hello sent, awaiting response.");

    Ok(())
}

fn receive_welcome(stream: &Mutex<TcpStream>) -> Result<Welcome> {
    let mut data = [0 as u8; 512];
    let _n_bytes = stream.lock().unwrap().read_exact(&mut data)?;
    println!("data read by client following hello send: {:?}", &data);
    let welcome = if let Ok(
                            MessageSC::ReplyMessageSC(
                                ReplySC::ReplyWelcomeSC(
                                    welcome))) = parse_array_to_enum(&data) {
        welcome
    } else {
        panic!("Uh oh! Received something other than a welcome message.");
    };

    println!("Welcome to Snowcast! The server has {} stations.",
              welcome.number_stations);
    Ok(welcome)
}

fn receive_set_station(stream: &Mutex<TcpStream>,
                       //active_stations: Arc<Mutex<HashMap<u16, Vec<u16>>>>,
                       hello: &Hello,
                       //file_vec: Vec<String>,
                       station_vec: &mut Vec<Station>,
                       /*ip: &Ipv4Addr*/) -> Result<()> {
    let mut data = [0 as u8; 512];

    stream.lock().unwrap().read_exact(&mut data)?;
    println!("data read by server anticipating set_station: {:?}", &data);
    match parse_array_to_enum(&mut data) {
        Ok(MessageSC::SendMessageSC(
                SendSC::SendSetStationSC(set_station))) => {
            //station_vec.insert(set_station.station_number, )
            //station_vec.iter().find_map(|song| )
            //station_vec[set_station.station_number] = 
            if let Some(station) = station_vec.get_mut(set_station.station_number as usize) {
                println!("Added port to station_vec");
                station.udp_ports.lock().unwrap().push(hello.udp_port);
            }
            
            // let key = match active_stations.lock()
            //                                .unwrap()
            //                                .iter()
            //                                .find_map(
            //                                    |(key, &ref vec)|
            //                                    if vec.contains(&hello.udp_port)
            //                                    { Some(key) } else { None }) {
            //     Some(old_key) => old_key.clone(),
            //     None => set_station.station_number.clone(),
            // };
            // match active_stations.lock().unwrap().entry(key) {
            //     Entry::Vacant(entry) => {
            //         entry.insert(vec![hello.udp_port]);
            //         let _ = broadcast_song(&file_vec[set_station.station_number as usize], ip, active_stations);
            //     }
            //     Entry::Occupied(mut entry) => {
            //         entry.get_mut().push(hello.udp_port);
            //     }
            // }
            Ok(())
            
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
    }
}

//pub fn player_daemon(song_file_vec: Vec<String>, active_stations: Arc<Mutex<HashMap<u16, Vec<u16>>>>) {
// pub fn player_daemon(station_vec: Vec<Station>, server_name: Ipv4Addr) {
//     //initialize threads playing given songs
// 
//     for song in station_vec.iter() {
//         let server_name_clone = server_name.clone();
//         thread::spawn(move||broadcast_song(song, *server_name_clone));
//     }
// 
// 
// 
//     // loop {
//     //     match station_vec.lock()
//     //                      .unwrap()
//     //                      .iter()
//     //                      .find_map(|song| if song.udp_ports.lock().unwrap().is_empty()
//     //                              { Some(()) } else { None }) {
//     //         Some(_) => {
//     //             println!("Found an active station!");
//     //             station_vec.lock().unwrap().iter().map(|song| )
//     //         }
//     //         None => {
//     //             thread::sleep(time::Duration::from_millis(500));
//     //             continue
//     //         }
//     //     };
//     // }
// }


//pub fn broadcast_song(song_path: &str,
//                      server_name: &Ipv4Addr,
//                      udp_port: Arc<Mutex<Vec<u16>>>)
//                      -> Result<()> {
// fn play_song(song: &Station, server_name: Ipv4Addr) -> Result<()> {
//     
//     // takes in a song and controls 
// 
// 
// 
//     println!("Printing from broadcast_song!");
//     //let full_ip = format!("{}:{}", server_name, udp_port.lock().unwrap());
//     //let socket_bind = UdpSocket::bind("127.0.0.1:7878").expect("Couldn't bind to address.");
//     //let socket_bind = Arc::new(Mutex::new(socket_bind));
// 
//     //let socket: UdpSocket = UdpSocket::bind("127.0.0.1:7878")?;
// 
//     for udp_port in song.udp_ports.lock().unwrap().iter() {
//         //let full_ip = format!("{}:{}", &server_name, udp_port);
//         //let _ = socket.connect(full_ip).unwrap();
//         let song_path_clone = song.song_path.clone();
//         let server_name_clone = server_name.clone();
//         let udp_port_clone = udp_port.clone();
// 
//         //let socket: Arc<Mutex<UdpSocket>> = Arc::new(Mutex::new(socket));
//         //let socket_clone = socket.clone();
//         //let socket_clone = socket.clone();
//         //let _ = socket_bind.lock().unwrap().connect(full_ip);
//         //let socket: Mutex<UdpSocket> = Mutex::new(socket_bind);
//         thread::spawn(move||play_to_udp(song_path_clone, server_name_clone, udp_port_clone));
//     }
//     Ok(())
// 
//     //      // 16384 bytes/second = 1024 bytes * 16 /sec  // MUST be < 1500 bytes/sec
//     //      // 1024 bytes every .0625 sec
//     //      //
//     //      // 16384 bytes/second = 64 bits or 8 bytes * 2048 / sec
//     //      // u64 every 0.00048828125
//     //      //
//     //      //let mut buf = [0 as u8; 8];
//     //      let mut file = File::open(song_path)?;
//     //      let time_gap = time::Duration::from_micros(62500);
//     //      //let time_gap_num_u64 = 488280;
//     //      //let time_gap = time::Duration::from_nanos(time_gap_num);
//     //      //let smaller_time_gap = time::Duration::from_nanos(time_gap_num_u64/8);
//     //      loop { //song loop
//     //          println!("starting song");
//     //          let _ = file.rewind();
//     //          let mut buf = [0 as u8; 1024];
//     //          'within_song: loop {
//     //              //println!("printing each loop of song");
//     //              match file.read_exact(&mut buf) {
//     //                  Ok(_) => {
//     //                      let _ = socket.send(&buf);
//     //                      thread::sleep(time_gap);
//     //                  }
//     //                  Err(error) => match error.kind() {
//     //                      ErrorKind::UnexpectedEof => {
//     //                          //panic!("testing to see if this is the right error");
//     //                          break 'within_song
//     //                      }
//     //                      _ => {
//     //                          panic!("unexpected error while reading {}\
//     //                          \nerror thrown: {}", song_path, error);
//     //                      }
//     //                  }
//     //              }
//     //          }
//     //      }
// }

// fn player_daemon(station_vec: Vec<Station>) {
//     // loops continuously while server is running
//     // checks udp_ports vec in each song to see if any new clients are connecting
//     //
//     
//     for song in station_vec {
// 
//     }
// 
//     loop {
//         station_vec.iter()
//                    .map(
//                        |song| if song.connection.udp_ports
//                        .lock().unwrap().is_empty() {
//                            song.
// 
// 
//                        } else {
// 
//                        })
// 
//     }
// }

pub fn play_all_songs(station_vec: Vec<Station>, server_name: Ipv4Addr) {
    for song in station_vec {
        play_song_looping(&song, server_name);
    }
}

fn play_song_looping(song: &Station, server_name: Ipv4Addr) {
    loop {
        if song.udp_ports.lock().unwrap().is_empty() {
            println!("no udp_ports yet for: {}", &song.song_path);
            thread::sleep(time::Duration::from_millis(500));
            // wait and see if arc mutex will contain values next time
        } else {
            for udp_port in song.udp_ports.lock().unwrap().iter() {
                let song_path_clone = song.song_path.clone();
                let server_name_clone = server_name.clone();
                let udp_port_clone = udp_port.clone();
                thread::spawn(move||play_to_udp_once(song_path_clone,
                                         server_name_clone,
                                         udp_port_clone));
            }
        }
    }
}

fn play_to_udp_once(song_path: String,
                       server_name: Ipv4Addr,
                       udp_port: u16) -> Result<()> {
    let mut file = File::open(&song_path)?;
    let time_gap = time::Duration::from_micros(62500);
    let socket = UdpSocket::bind("127.0.0.1:7878")?;
    //let _ = socket.connect("127.0.0.1:16801")?;
    let full_ip = format!("{}:{}", server_name, udp_port);
    let _ = socket.connect(full_ip);
    //let time_gap_num_u64 = 488280;
    //let time_gap = time::Duration::from_nanos(time_gap_num);
    //let smaller_time_gap = time::Duration::from_nanos(time_gap_num_u64/8);
    println!("starting song: {}", &song_path);
    //todo!(announce()); //TODO
    let mut buf = [0 as u8; 1024];
    loop{
        //println!("printing each loop of song");
        match file.read_exact(&mut buf) {
            Ok(_) => {
                let _ = socket.send(&buf);
                thread::sleep(time_gap);
            }
            Err(error) => match error.kind() {
                ErrorKind::UnexpectedEof => {
                    //panic!("testing to see if this is the right error");
                    break
                }
                _ => {
                    panic!("unexpected error while reading {}\
                    \nerror thrown: {}", &song_path, error);
                }
            }
        }
    }
    Ok(())
}


// fn play_to_udp(song_path: String, server_name: Ipv4Addr, udp_port: Option<u16>) -> Result<()> {
//     // match udp_port {
//     //     Some(udp_port) => {
//     //         
//     //     }
//     // }
// 
// 
// 
//     let mut file = File::open(&song_path)?;
//     let time_gap = time::Duration::from_micros(62500);
//     let socket = UdpSocket::bind("127.0.0.1:7878")?;
//     //let _ = socket.connect("127.0.0.1:16801")?;
//     let full_ip = format!("{}:{}", server_name, udp_port);
//     let _ = socket.connect(full_ip);
//     //let time_gap_num_u64 = 488280;
//     //let time_gap = time::Duration::from_nanos(time_gap_num);
//     //let smaller_time_gap = time::Duration::from_nanos(time_gap_num_u64/8);
//     loop { //song loop
//         println!("starting song");
//         let _ = file.rewind();
//         let mut buf = [0 as u8; 1024];
//         'within_song: loop{
//             //println!("printing each loop of song");
//             match file.read_exact(&mut buf) {
//                 Ok(_) => {
//                     let _ = socket.send(&buf);
//                     thread::sleep(time_gap);
//                 }
//                 Err(error) => match error.kind() {
//                     ErrorKind::UnexpectedEof => {
//                         //panic!("testing to see if this is the right error");
//                         break 'within_song
//                     }
//                     _ => {
//                         panic!("unexpected error while reading {}\
//                         \nerror thrown: {}", &song_path, error);
//                     }
//                 }
//             }
//         }
//     }
// }


// play_song()

// basic_udp_streamer(song_path, server_name, udp_port)
//  binds and connects to server_name:udp_port, if no port, handle error gracefully and wait
//  if udp_port exists, sends data of song_path to stream, no error checking
