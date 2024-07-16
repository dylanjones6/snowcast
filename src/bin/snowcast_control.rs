use std::env;
use std::io::/*{Read, Write, */Result/*}*/;
use std::net::/*{*/Ipv4Addr/*, TcpStream}*/;
use snowcast::structs::{initiate_handshake, set_station};

fn main() -> Result<()> {
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
    let udp_port = if let Ok(_) = args3_parse {
        args3_parse.unwrap()
    } else {
        panic!("The third argument (<udp_port>) must be an integer \
                from 0 to 65535");
    };


    // let server_name = &args[1]; // TODO IMPLEMENT INPUT CHECKS!!!!
    // let server_port = &args[2];
    // let udp_port = &args[3];
    initiate_handshake(&server_name, &server_port, &udp_port);

    /*let stationNum = 'input: */loop {
        println!("What station would you like to select? If you're done, \
                  press \"q\" to exit.");
        let mut input = String::new();
        let _ = std::io::stdin().read_line(&mut input);
        let input: Vec<String> = input.trim().split_whitespace().map(String::from).collect();
        //println!("{:?}", input);
        let station_number = if input.len() == 1 && input[0] == "q" {
            std::process::exit(1);
        } else if input.len() != 1 || input[0].parse::<u16>().is_err() {
            eprintln!("Pick a station from 0 to 65535 or quit with \"q\".");
            continue// 'input
        } else {
            input[0].parse::<u16>().unwrap()
        };
        set_station(&station_number);

        println!("selected station {}", &station_number);
        //structs::SetStation(&station_number) // uncomment this at some point!
        //break stationNum
        //if stationNum.//EXISTS!
    };
    //SetStation(&stationNum);
}

