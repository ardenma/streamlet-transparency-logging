
use super::network;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use rand::Rng;

use log::info;

#[derive(Debug)]
pub struct Peers {
    // Public -- everyone can access name
    pub node_name: String,
    // Public for ease of lookup, iteration, etc. 
    pub peer_list: HashMap<String, Vec<u8>>,
    num_expected: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PeerAdvertisement {
    end_init: bool,
    node_name: String, 
    public_key: Vec<u8>,
    known_peers: Vec<String>,
}

impl Peers {
    /* Initializer: 
        @param my_name: identifying "name" of this node 
            (if empty string, name will be generated from random 32-bit number)
     */
    pub fn new(mut my_name: String) -> Self {
        if my_name == "" { 
            let rand : u32 = rand::thread_rng().gen();
            my_name = format!("{}", rand).to_string();
        }
        info!("Initializing peer with name {}", my_name);
        Self{ node_name: my_name, peer_list: HashMap::new(), num_expected: 0 }
    }

    /* Start (or restart) an initialization process from scratch. 
       @param net_stack: network stack containing an initialization channel to send on. 
       @param expected_count: the number of peers we expect to receive during initialization.
                              NOT including the current node.
       Note: this can be called again to force a re-initialization. */
    pub fn start_init(&mut self, net_stack: &mut network::NetworkStack, expected_count: usize) {
        if net_stack.init_channel_open() {
            return;
        }
        info!("{} is starting initialization; adding {} peers", self.node_name, expected_count);
        self.num_expected = expected_count;
        self.peer_list = HashMap::new();
        net_stack.open_init_channel();
        self.advertise_self(net_stack);
    }

    /* Should be triggered whenever a PeerAdvertisement is received from the network. 
       Inserts the peer into the hashmap if not already present. 
       Protocol will accept first public key received for a peer. 
       Closes the initialization channel if all peers have been received. 

       @param ad: PeerAdvertisement received from the network 
       @param net_stack: network stack containing an initialization channel to send on. */
    pub fn recv_advertisement(&mut self, ad: PeerAdvertisement, net_stack: &mut network::NetworkStack) {
        if ad.end_init && self.is_done() {
            self.end_init(net_stack);
            return; 
        }
        if self.is_done() || self.peer_list.contains_key(&ad.node_name) {
            info!("{} received a duplicate or out-of-scope peer advertisement", self.node_name);
            return;
        }
        info!("{} adding peer: {}", self.node_name, ad.node_name);
        self.peer_list.insert(ad.node_name, ad.public_key);
        
        if !ad.known_peers.contains(&self.node_name) {
            self.advertise_self(net_stack);
        }

        if self.is_done() && net_stack.init_channel_open() {
            info!("{} is done with peer discovery protocol; discovered {} peer(s)", 
                    self.node_name, 
                    self.peer_list.len());
        }
    }

    /* If all expected advertisements have been received. */
    pub fn is_done(&self) -> bool {
        self.peer_list.len() >= self.num_expected
    }

    pub fn send_end_init(&mut self, net_stack: &mut network::NetworkStack) {
        let my_ad = PeerAdvertisement{ 
            end_init: true,
            node_name: String::new(), 
            public_key: Vec::new(), 
            known_peers: Vec::new(),
        };
        net_stack.send_init_channel(serde_json::to_vec(&my_ad).expect("Can't create advertisement."));
        self.end_init(net_stack);
    }

    /*  Close the initialization channel. 
    Should not need to be called externally -- will be closed after the "last" 
    advertisement is received. */
    fn end_init(&mut self, net_stack: &mut network::NetworkStack) {
        if !net_stack.init_channel_open() {
            return; 
        }
        info!("ending init protocol");
        info!("Final state: {:?}", self);
        net_stack.close_init_channel();
    }

    /* May be useful, e.g., if you restart with one additional peer. */
    #[allow(dead_code)]
    pub fn num_peers_expected(&self) -> usize {
        self.num_expected
    }

    /* May be useful, e.g., if we discover a peer is malicious. */
    #[allow(dead_code)]
    pub fn permanently_delete_peer(&mut self, name: String) {
        // If value was in the map, expected count should go down
        if let Some(_) = self.peer_list.remove(&name) {
            self.num_expected -= 1;
        }
    }

    fn advertise_self(&mut self, net_stack: &mut network::NetworkStack) {
        let my_ad = PeerAdvertisement{ 
            end_init: false,
            node_name: self.node_name.clone(), 
            public_key: Vec::new(), 
            known_peers: Vec::from_iter(self.peer_list.keys().cloned() ),
        };
        net_stack.send_init_channel(serde_json::to_vec(&my_ad).expect("Can't create advertisement."));
    }

}
