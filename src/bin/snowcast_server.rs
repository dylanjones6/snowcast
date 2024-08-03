use std::thread;
use std::net::{Ipv4Addr, TcpListener, TcpStream};
//use snowcast::structs::{self, all_station_player, Station};
use snowcast::structs::{self, play_all_loops, handle_client, Station};
use std::sync::{Arc, Mutex, RwLock};
use std::io::Result;
use std::io::ErrorKind;
use std::fs::File;

/// takes no arguments and uses std::env::args() to collect tcp_port and file
/// info. returns a tuple containing (tcp_port: u16, files: Vec<&str>)
fn get_args() -> Result<(u16, Vec<String>), > {
    let args: Vec<String> = std::env::args().collect();
    //let args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

    if args.len() < 3 {
        eprintln!("Usage: ./snowcast_server <tcp_port> <file0> [file1] [file2] \
                   ...");
        return Err(ErrorKind::InvalidData.into())
        //std::process::exit(1);
    }
    let tcp_port = if let Ok(port) = args[1].parse::<u16>() {
        port
    } else {
        eprintln!("The first argument must be an int from 0 to 65535");
        return Err(ErrorKind::InvalidInput.into())
        //std::process::exit(1);
    };
    let song_path_vec: Vec<String> = args[2..].to_vec();
    //let file_vec: Vec<&str> = args[2..].to_vec();

    Ok((tcp_port, song_path_vec))
}

fn main() -> std::io::Result<()> /*-> Result<TcpListener, _>*/ {
    let (tcp_port, song_path_vec) = get_args()?;

    let server_name = "127.0.0.1".parse::<Ipv4Addr>().unwrap();
    let server_udp = "7878".parse::<u16>().unwrap();
    //let station_vec: Arc<Mutex<Vec<Station>>> = Arc::new(Mutex::new(Vec::new()));
    //for song in song_path_vec.clone() {
    //    let station_temp: Station = Station::new(song, Vec::new())?;
    //    station_vec.lock().unwrap().push(station_temp)
    //}
     let mut station_vec: Vec<Station> = Vec::new();
    for song in song_path_vec.clone() {
        let station_temp: Station = Station::new(song, Vec::new())?;
        station_vec.push(station_temp)
    }

    let _ = play_all_loops(server_name, server_udp, station_vec.clone());

    let listener = TcpListener::bind(format!("{}:{}", &server_name, &tcp_port))?;
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("New connection: {}", &stream.peer_addr().unwrap());
                let song_path_vec_clone = song_path_vec.clone();
                let mut station_vec_clone = Vec::new();
                for i in &station_vec {
                    let i = i.clone();
                    station_vec_clone.push(i);
                }
                thread::spawn(move || {
                    structs::handle_client(Arc::new(RwLock::new(stream)), song_path_vec_clone, station_vec_clone)
                });
                //println!("connection ended with {}", &stream_peer_add_copy)
            }
            Err(error) => {
                eprintln!("An error occurred while accepting stream: {}", error);
            }
        }
    }
    Ok(())
}
