use crate::utils::crypto::*;
use crate::network::NetworkStack;
use crate::messages::*;

use bincode::{deserialize, serialize};
use serde::{Deserialize, Serialize};

pub struct AppInterface {
    app_public_key: Option<PublicKey>,
}

pub const APP_SENDER_ID: u32 = 0;
pub const APP_NET_TOPIC: &'static str = "app";


impl AppInterface {
    
    /* Wrapper - meant to keep the network stack general, while giving 
       an easy method to call to exchange dat with the app. */
    pub fn new(net_stack: &mut NetworkStack) -> Self {
        net_stack.add_topic(APP_NET_TOPIC);
        Self { app_public_key: None }
    }

    pub fn init(&mut self, public_key: PublicKey) {
        self.app_public_key = Some(public_key);
    }

    pub fn send_to_app(&self, net_stack: &mut NetworkStack, msg: Vec<u8>) {
        net_stack.broadcast_to_topic(APP_NET_TOPIC, msg);
    }

    /* CUSTOMIZABLE: should this data be accepted? */
    pub fn data_is_valid(&self, _message: &Message) -> bool {
        true

        // TODO
        /* if message.signatures.is_none() || message.signatures.unwrap().len() != 1 {
            return false;
        }
        let signature = message.signatures.unwrap()[1];
        if let MessagePayload::AppData(appdata) = message.payload {
            if let Ok(()) = self.app_public_key.verify(appdata, signature) {
                return true;
            } else {
                return false; 
            }
        } else {
            false
        }
    } */
    }
}