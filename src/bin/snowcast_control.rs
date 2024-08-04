use std::env;
use std::io::Result;
use std::net::Ipv4Addr;
use tokio::net::TcpStream;
use tokio::sync::{Mutex, RwLock};
use std::sync::Arc;
//use snowcast::structs::{initiate_handshake, set_station};
use snowcast::structs::interact_with_server;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    //let args = &args[1..];
    // dbg!(args);
    if args.len() != 4 {
        eprintln!("Usage: ./snowcast_control <server_name> <server_port> \
                   <udp_port>");
        std::process::exit(1);
    }
    let args1_parse = args[1].parse::<Ipv4Addr>();
    let server_name = if args[1] == "localhost" {
        "127.0.0.1".parse::<Ipv4Addr>().unwrap() //this is ok to unwrap bc we 
                                                 //check localhost
    } else if let Ok(_) = args1_parse {
        args1_parse.unwrap()
    } else {
        panic!("The first argument must be a valid IPv4 address \
                  (ex: 192.168.0.1) or localhost");
    };

    let args2_parse = args[2].parse::<u16>();
    let server_port = if let Ok(_) = args2_parse {
        args2_parse.unwrap()
    } else {
        panic!("The second argument (<server_port>) must be an integer \
                from 0 to 65535");
    };

    let args3_parse = args[3].parse::<u16>();
    let client_udp_port = if let Ok(_) = args3_parse {
        args3_parse.unwrap()
    } else {
        panic!("The third argument (<udp_port>) must be an integer \
                from 0 to 65535");
    };

    //TODO improve argument parsing bc this is dogshit

    let full_address = format!("{}:{}", server_name, server_port);
    println!("{}", &full_address);
    //println!("test");

    let stream = Arc::new(RwLock::new(TcpStream::connect(&full_address).await?));

    println!("Connected to server at {}", &full_address);

    let _ = interact_with_server(stream, client_udp_port).await;
    Ok(())

    //let welcome = initiate_handshake(&stream, &udp_port)?;

    //loop {
    //    println!("What station would you like to select? If you're done, \
    //              press \"q\" to exit.");
    //    let mut input = String::new();
    //    let _ = std::io::stdin().read_line(&mut input);
    //    let input: Vec<String> = input.split_whitespace().map(String::from).collect();
    //    //println!("{:?}", input);
    //    let station_number = if input.len() == 1 && input[0] == "q" {
    //        std::process::exit(1);
    //    } else if input.len() != 1 || input[0].parse::<u16>().is_err() {
    //        eprintln!("Pick a station from 0 to {} or quit with \"q\".", welcome.number_stations);
    //        continue// 'input
    //    } else {
    //        input[0].parse::<u16>().unwrap()
    //    };

    //    set_station(&stream, station_number)?;
    //    println!("selected station {}", &station_number);
    //};
}

