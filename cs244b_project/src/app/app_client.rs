use crate::app::*;
use tokio::{
    io::{stdin, AsyncBufReadExt, AsyncWriteExt, AsyncReadExt, BufReader},
    select,
    sync::{mpsc, Mutex},
    net::{TcpStream},
};
use log::{error, info};
use bincode::{deserialize, serialize};


pub struct Client;

impl Client {
    pub fn new() -> Self {
        Self
    }

    pub async fn run(&self) { 
        let mut stdin = BufReader::new(stdin()).lines();
        loop {
            let _ = stdin.next_line().await.expect("Can't read from stdin");
            self.get_data().await;
        }

    }
    
    async fn get_data(&self) {
        // SERVER_IP
        let res = TcpStream::connect(SERVER_IP).await;
        if let Err(e) = res {
            error!("Can't connect to server: {:?}", e);
            return;
        }
        info!("Connected to server");
        let mut stream = res.unwrap();
        let mut message = vec![0; 10000];

        let res = stream.read(&mut message[..]).await;
        if let Err(e) =  res {
            error!("Error reading from stream: {:?}", e);
            return;
        }
        message.truncate(res.unwrap());

        let res = deserialize::<ResponseToClient>(&message);

        if let Err(e) = &res {
            error!("Error deserializing: {:?}", e);
            return;
        }

        let resp = res.unwrap();
        match resp {
            ResponseToClient::Response(dir) => {
                println!("Received directory: {}", dir);
            },
            ResponseToClient::NoneAvailable => {
                println!("No directory available");
            },
        }
    }  
}