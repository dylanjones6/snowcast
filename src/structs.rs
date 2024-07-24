use std::net::{TcpStream, UdpSocket, Ipv4Addr};
use std::io::{ErrorKind, Read, Result, Seek, Write};
use std::fs::File;
use std::sync::{Mutex, Arc};
use std::{thread, time};
//use crate::structs;
use byteorder::{ByteOrder, NetworkEndian};


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
    station_vec: Vec<Station>) -> Result<()> {

    let hello: Hello = receive_hello(&stream)?;

    //let file_vec_clone = file_vec.clone();
    let number_stations: u16 = file_vec.len().try_into().unwrap();

    let _ = send_welcome(&stream, number_stations);

    //let mut data = [0 as u8; 512];

    loop {
        //let station_vec_clone = station_vec.clone();
        let _ = receive_set_station(&stream, &hello, station_vec.clone());
    }
}


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
                     ) -> Result<()> {
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
            return Ok(())
        }
        Err(error) => {
            panic!("Error caused while set_station wrote to stream: {}",
                error);
        }
    }
}

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
                       hello: &Hello,
                       station_vec: Vec<Station>,
                       /*ip: &Ipv4Addr*/) -> Result<()> {
    let mut data = [0 as u8; 512];

    stream.lock().unwrap().read_exact(&mut data)?;
    println!("data read by server anticipating set_station: {:?}", &data);
    match parse_array_to_enum(&mut data) {
        Ok(MessageSC::SendMessageSC(
                SendSC::SendSetStationSC(set_station))) => {
            println!("hey you're close!");
            if let Some(station) = station_vec.get(set_station.station_number as usize) {
                println!("id be amazed if this printed");
                station.udp_ports.lock().unwrap().push(hello.udp_port);
            }
            println!("added port to station_vec");
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

fn send_announce(stream: &Mutex<TcpStream>,
                 songname_length: u8,
                 songname: [u8; 256]) -> Result<()> {


}

pub fn play_loop(station_vec: Vec<Station>,
              server_name: Ipv4Addr,
              open_file_vec: Arc<Mutex<Vec<File>>>) -> Result<()> {
    loop {
        let _ = play_all_songs_chunk(station_vec.clone(), server_name, open_file_vec.clone());
    }
}

fn play_all_songs_chunk(station_vec: Vec<Station>,
                        server_name: Ipv4Addr,
                        open_file_vec: Arc<Mutex<Vec<File>>>) -> Result<()> {
    let time_gap = time::Duration::from_micros(62500);
    //let smaller_time_gap = time::Duration::from_millis(5);
    for (i, song) in station_vec.iter().enumerate() {
        //let file = File::open(song.song_path)?;
        let mut song_buf = [0 as u8; 1024];
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
