use std::thread;
use std::net::TcpListener;
use snowcast::structs;

fn main() -> std::io::Result<()> /*-> Result<TcpListener, _>*/ {
    let ip = "127.0.0.1";
    //let tcp_port = "16800";
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: ./snowcast_server <tcp_port> <file0> [file1] [file2] \
                   ...");
        std::process::exit(1);
    }
//let tcp_port = if args[1].parse::<u16>().is_ok() { &args[1]
    let tcp_port = if let Ok(port) = args[1].parse::<u16>() {
        port
    } else {
        eprintln!("The first argument must be an int from 0 to 65535");
        std::process::exit(1);
    };
    let file_vec: Vec<String> = (&args[2..]).to_vec();
    let number_stations: u16 = file_vec.len(); //TODO implement number_stations into response

    let listener = TcpListener::bind(format!("{}:{}", &ip, &tcp_port))?;
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("New connection: {}", &stream.peer_addr().unwrap());
                //let stream_peer_add_copy = &stream.peer_addr().unwrap();
                let file_vec_clone = file_vec.clone();
                thread::spawn(move|| {
                    structs::handle_client(stream, &number_stations)
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
