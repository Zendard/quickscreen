use crate::host::network::{
    CLIENT_TO_HOST_MESSAGE_SIZE, ClientID, ClientToHostNetworkMessage, HOST_TO_CLIENT_MESSAGE_SIZE,
    HostToClientNetworkMessage,
};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    sync::mpsc::{Receiver, Sender},
};

pub enum JoinedToUIMessage {
    JoinRequestResponse(bool),
}
pub enum UIToJoinedMessage {
    Leave,
}

pub fn join(
    address: IpAddr,
    port: u16,
    message_sender: Sender<JoinedToUIMessage>,
    message_receiver: Receiver<UIToJoinedMessage>,
) {
    let udp_socket =
        UdpSocket::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port + 1)).unwrap();
    udp_socket.connect(SocketAddr::new(address, port)).unwrap();

    let id = ClientID::generate();
    let network_buffer: [u8; CLIENT_TO_HOST_MESSAGE_SIZE] =
        ClientToHostNetworkMessage::JoinRequest(id).into();
    udp_socket.send(&network_buffer).unwrap();

    udp_socket.set_nonblocking(true).unwrap();

    let mut network_buffer = [0; HOST_TO_CLIENT_MESSAGE_SIZE];
    loop {
        if let Ok(message) = message_receiver.try_recv() {
            match message {
                UIToJoinedMessage::Leave => break,
            }
        }

        if let Ok(bytes_amount) =
            udp_socket.peek(&mut [0; std::mem::size_of::<HostToClientNetworkMessage>()])
            && bytes_amount >= std::mem::size_of::<HostToClientNetworkMessage>()
        {
            udp_socket.recv(&mut network_buffer).unwrap();
            let network_message = HostToClientNetworkMessage::try_from(network_buffer);
            if network_message.is_err() {
                continue;
            }

            handle_network_message(network_message.unwrap(), &message_sender);
        }
    }
    println!("Leaving...");

    let network_buffer: [u8; CLIENT_TO_HOST_MESSAGE_SIZE] =
        ClientToHostNetworkMessage::Left(id).into();
    udp_socket.send(&network_buffer).unwrap();
}

fn handle_network_message(
    message: HostToClientNetworkMessage,
    message_sender: &Sender<JoinedToUIMessage>,
) {
    match message {
        HostToClientNetworkMessage::JoinRequestResponse(accepted) => {
            handle_join_request_response(accepted, message_sender)
        }
        HostToClientNetworkMessage::Frame(frame) => return,
    }
}

fn handle_join_request_response(accepted: bool, message_sender: &Sender<JoinedToUIMessage>) {
    if accepted {
        println!("We were accepted")
    } else {
        println!("We were refused")
    }
    message_sender
        .send(JoinedToUIMessage::JoinRequestResponse(accepted))
        .unwrap();
}
