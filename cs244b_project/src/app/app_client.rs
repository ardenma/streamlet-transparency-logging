use crate::app::*;
use tokio::{
    io::{stdin, AsyncBufReadExt, AsyncWriteExt, AsyncReadExt, BufReader},
    select,
    sync::{mpsc, Mutex},
    net::{TcpStream},
};
use log::{error, info};
use bincode::{deserialize, serialize};

// TODO: test this
const PADDING : usize = 1000;
const MSG_SIZE : usize = OnionRouterNetDirectory::MAX_SIZE 
                       * std::mem::size_of::<OnionRouterBasicData>()
                       + PADDING;

/*** Basic Tor client meant to interact with directory service. ***/

pub struct Client;

impl Client {
    pub fn new() -> Self {
        Self
    }

    /** Infinite loop **/
    pub async fn run(&self) { 
        let mut stdin = BufReader::new(stdin()).lines();
        loop {
            // Starting point: everything => get data from directory server
            let _ = stdin.next_line().await.expect("Can't read from stdin");
            self.get_data().await;
        }

    }
    
    // Open TCP connection to server, send request for directory, and print response
    async fn get_data(&self) {
        let res = TcpStream::connect(SERVER_IP).await;
        if let Err(e) = res {
            // Sometimes there are legit reasons for this -- no need to crash
            error!("Can't connect to server: {:?}", e);
            return;
        }
        let mut stream = res.unwrap();
        // Allocate a large vector 
        let mut message = vec![0; MSG_SIZE];

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