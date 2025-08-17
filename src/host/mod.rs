use self::network::ClientToHostNetworkMessage;
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    sync::mpsc::{Receiver, Sender},
};

mod capture;
pub mod network;

pub enum HostingToUIMessage {}
pub enum UIToHostingMessage {
    Stop,
}

pub fn host(
    port: u16,
    message_sender: Sender<HostingToUIMessage>,
    message_receiver: Receiver<UIToHostingMessage>,
) {
    let mut capturer = capture::new().unwrap();
    capturer.start_capture();

    let socket = UdpSocket::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port)).unwrap();
    let mut network_buffer = [0; std::mem::size_of::<ClientToHostNetworkMessage>() + 1];

    loop {
        if let Ok(message) = message_receiver.try_recv() {
            match message {
                UIToHostingMessage::Stop => break,
            }
        }
        // let frame = capturer.get_next_frame().unwrap();

        let bytes_amount = socket
            .peek(&mut [0; std::mem::size_of::<ClientToHostNetworkMessage>()])
            .unwrap();
        if bytes_amount >= std::mem::size_of::<ClientToHostNetworkMessage>() {
            socket.recv(&mut network_buffer).unwrap();
            dbg!(bytes_amount);
            dbg!(&network_buffer);
            let network_message = ClientToHostNetworkMessage::try_from(network_buffer);
            dbg!(&network_message);
        }
    }
    capturer.stop_capture();
    println!("Stopped hosting");
}
