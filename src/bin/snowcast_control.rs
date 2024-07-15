use std::env;
use std::io::{Read, Write, Result};
use std::net::{server_namev4Addr, Ipv4Addr, TcpStream};
use snowcast::structs::initiate_handshake;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    //let args = &args[1..];
    // dbg!(args);
    if args.len() != 4 {
        eprintln!("Usage: ./snowcast_control <server_name> <server_port> \
                   <udp_port>");
        std::process::exit(1);
    }
    let server_name = if args[1] == "localhost" {
        "127.0.0.1".parse::<Ipv4Addr>().unwrap() //this is ok to unwrap bc we 
                                                 //check localhost
    } else if let Ok(ip) = args[1].parse::<Ipv4Addr>() {
        ip
    } else {
        panic!("The first argument must be a valid IPv4 address \
                  (ex: 192.168.0.1) or localhost)");
    };

    let server_port = if let Ok(port) = args[2].parse<u16>() {
        port
    } else {
        panic!("The second argument must be an integer from 0 to 65535");
    };

    let udp_port = if let Ok(port) = args[3].parse<u16>() {
        port
    } else {
        panic!("The third argument must be an integer from 0 to 65535");
    };


    let server_name = &args[1]; // TODO IMPLEMENT INPUT CHECKS!!!!
    let server_port = &args[2];
    let udp_port = &args[3];
    initiate_handshake(server_name, server_port, udp_port);

    /*let stationNum = 'input: */loop {
        println!("What station would you like to select? If you're done \
                  press \"q\" to exit.");
        let mut input = String::new();
        let _ = std::io::stdin().read_line(&mut input);
        let input: Vec<String> = input.trim().split_whitespace().map(String::from).collect();
        //println!("{:?}", input);
        let station_num = if input.len() == 1 && input[0] == "q" {
            std::process::exit(1);
        } else if input.len() != 1 || input[0].parse::<u16>().is_err() {
            eprintln!("Pick a station from 0 to 65535 or quit with \"q\".");
            continue// 'input
        } else {
            input[0].parse::<u16>().unwrap()
        };
        //SetStation(&stationNum)
        println!("selected station {}", &station_num);
        //structs::SetStation(&station_num) // uncomment this at some point!
        //break stationNum
        //if stationNum.//EXISTS!
    };
    //SetStation(&stationNum);
}

