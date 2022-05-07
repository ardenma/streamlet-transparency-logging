use std::env;
use std::net::{TcpListener, TcpStream};
use std::{thread, time};
use std::io::{Read, Write};
use std::str;

const N: i32 = 4;

#[derive(Debug)]
struct ConfigData {
    my_ip: String,
    peer_ips: Vec<String>,
}


// This should probably read from a file. 
fn set_config(config: &mut ConfigData, my_name: String) {
    
    for i in 0..N {
        if my_name == format!("h{}", i + 1) {
            config.my_ip = format!("10.0.0.{}", i + 1);
        }
        else {
            config.peer_ips.push(format!("10.0.0.{}", i + 1));
        }
    }
}


// Receive a message and print it 
fn handle_message(mut stream: TcpStream, my_ip: &String) {
    let mut buf = [0_u8; 100];
    let mut bytes_read = 0;
    loop {
        let new_bytes = stream.read(&mut buf[bytes_read..]).unwrap();
        if new_bytes == 0 {
            break;
        };
        bytes_read += new_bytes;
    }
    let s = str::from_utf8(&buf).unwrap();
    println!("{} received from {}: {}", my_ip, stream.peer_addr().unwrap().ip().to_string(), s);
    
}


fn open_sockets(config: &ConfigData) {
    let my_ip = config.my_ip.clone();
    thread::spawn( move || {
       let listener = match TcpListener::bind(format!("{}:80", my_ip)) {
           Ok(listener) => listener,
           Err(_err) => {
               println!("error binding locally");
               std::process::exit(1);
           }
       };
       println!("Listening for requests on {}:80", my_ip);
       for stream in listener.incoming() {
           handle_message(stream.unwrap(), &my_ip);
       }
    } );

    // Wait for everyone else to get set up
    thread::sleep(time::Duration::from_secs(5));

    // Start sending messages
    let msg = format!("Hello from {}", config.my_ip).into_bytes();
    for peer_ip in &config.peer_ips {
        let res = TcpStream::connect(format!("{}:80", peer_ip));
        if let Err(e) = &res {
            println!("{} failed to connect to {}", config.my_ip, peer_ip);
            println!("{:?}", e);
            continue;
        }
        let mut conn = res.unwrap();
        // println!("Local address for {}: {}:{}", config.my_ip, conn.local_addr().unwrap().ip(), conn.local_addr().unwrap().port());
        // Write to stream
        conn.write_all(&msg).unwrap(); 
    }


}


fn main() {
    let mut my_config = ConfigData{ my_ip: "".to_string(), peer_ips: Vec::<String>::new() };
    let args: Vec<String> = env::args().collect();
    
    set_config(&mut my_config, args.get(1).cloned().unwrap());

    // println!("Hello, world from {}", args[1]);

    // println!("Config: {:?}", my_config);

    open_sockets(&my_config);

}
