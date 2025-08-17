use crate::host::network::{ClientID, ClientToHostNetworkMessage};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    sync::mpsc::{Receiver, Sender},
};

pub enum JoinedToUIMessage {}
pub enum UIToJoinedMessage {
    Leave,
}

pub fn join(
    address: IpAddr,
    port: u16,
    message_sender: Sender<JoinedToUIMessage>,
    message_receiver: Receiver<UIToJoinedMessage>,
) {
    let socket =
        UdpSocket::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port + 1)).unwrap();
    socket.connect(SocketAddr::new(address, port)).unwrap();
    let id = ClientID::generate();
    let network_buffer: [u8; 3] = ClientToHostNetworkMessage::JoinRequest(id).into();
    dbg!(&network_buffer);
    socket.send(&network_buffer).unwrap();
    loop {
        if let Ok(message) = message_receiver.try_recv() {
            match message {
                UIToJoinedMessage::Leave => break,
            }
        }
    }
    println!("Leaving...");
}
