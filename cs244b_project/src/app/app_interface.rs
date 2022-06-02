/* A customizable interface between the application (e.g., a network directory) and the Streamlet instance, 
   along with data meant to be shared between the application and the Streamlet instance. 
   Note: this is currently very bare-bones; it's meant to show what the API is. */

use crate::network::NetworkStack;
use crate::messages::*;

pub struct AppInterface;

pub const APP_SENDER_ID: u32 = 0;
pub const APP_NET_TOPIC: &'static str = "app";
pub const APP_NAME: &'static str = "app";


impl AppInterface {
    
    /* Wrappers - meant to keep the network stack general, while giving 
       an easy method to call to exchange data with the app. */
    pub fn new(net_stack: &mut NetworkStack) -> Self {
        net_stack.add_topic(APP_NET_TOPIC);
        Self
    }

    pub fn send_to_app(&self, net_stack: &mut NetworkStack, msg: Vec<u8>) {
        net_stack.broadcast_to_topic(APP_NET_TOPIC, msg);
    }

    /* CUSTOMIZABLE: Do we consider this message to be from the application? */
    pub fn message_is_from_app(&self, message: &Message) -> bool {
        return message.kind == MessageKind::AppSend &&
            message.sender_id == APP_SENDER_ID &&
            message.sender_name == APP_NAME.to_string() &&
            self.data_is_valid(message);
    }

    /* CUSTOMIZABLE: should this data be accepted? Do we consider it to be from the app? 
       We recommend, but do not currently implement for this proof-of-concept, 
       cryptographic signature validation here. */
    pub fn data_is_valid(&self, _message: &Message) -> bool {
        /* Dummy implementation for this proof-of-concept; just meant to show the API */
        true
    }
}