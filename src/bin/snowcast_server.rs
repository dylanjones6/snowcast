use std::thread;
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use snowcast::structs::{self, play_loop, Station};
use std::sync::{Mutex, Arc};
use std::io::Result;
use std::io::ErrorKind;
use std::fs::File;

/// takes no arguments and uses std::env::args() to collect tcp_port and file
/// info. returns a tuple containing (tcp_port: u16, files: Vec<&str>)
fn get_args() -> Result<(u16, Vec<String>), > {
    let args: Vec<String> = std::env::args().into_iter().collect();
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
    let file_vec: Vec<String> = args[2..].to_vec();
    //let file_vec: Vec<&str> = args[2..].to_vec();

    Ok((tcp_port, file_vec))
}

fn main() -> std::io::Result<()> /*-> Result<TcpListener, _>*/ {
    let (tcp_port, file_vec) = get_args()?;

    let server_name = "127.0.0.1".parse::<Ipv4Addr>().unwrap();
    //let tcp_port = "16800";
    let mut open_file_vec = Vec::new();

    let mut station_vec = Vec::new();
    for file_path in &file_vec {
        station_vec.push(Station::new(file_path.to_owned(), Arc::new(Mutex::new(Vec::new()))));
        open_file_vec.push(File::open(file_path).unwrap());
    }

    let open_file_vec = Arc::new(Mutex::new(open_file_vec));
    //let station_vec: Arc<Mutex<Vec<Station>>> = Arc::new(Mutex::new(station_vec));
    let station_vec_clone = station_vec.clone();
    //let station_vec_clone = station_vec


    thread::spawn(move|| play_loop(station_vec_clone, server_name, open_file_vec));


    //let number_stations: u16 = file_vec.len(); //TODO implement number_stations into response
    println!("listening test");
    let listener = TcpListener::bind(format!("{}:{}", &server_name, &tcp_port))?;
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("New connection: {}", &stream.peer_addr().unwrap());
                //let stream_peer_add_copy = &stream.peer_addr().unwrap();
                //let file_vec_clone = file_vec.clone();
                //let (tx, rx) = mpsc::channel();
                let stream: Mutex<TcpStream> = Mutex::new(stream);
                //let active_stations = Arc::new(Mutex::new(HashMap::new()));
                let file_vec_clone = file_vec.clone();
                let station_vec_clone = station_vec.clone();
                //let station_vec_clone = station_vec.clone();

                thread::spawn(move|| {
                    structs::handle_client(stream, server_name, file_vec_clone, station_vec_clone)
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
