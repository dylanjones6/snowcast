use std::thread;
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use snowcast::structs::{self, all_station_player, Station};
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

    let listener = TcpListener::bind(format!("{}:{}", &server_name, &tcp_port))?;
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("New connection: {}", &stream.peer_addr().unwrap());

                thread::spawn(move|| {
                    structs::handle_client()
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
