use crate::host::network::{
    ClientID, ClientToHostNetworkMessage, HOST_TO_CLIENT_MESSAGE_SIZE, HostToClientNetworkMessage,
    LargeSend, MAX_UDP_SEND_SIZE,
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
    let network_buffer: Vec<u8> = ClientToHostNetworkMessage::JoinRequest(id).into();
    udp_socket.send(&network_buffer).unwrap();

    udp_socket.set_nonblocking(true).unwrap();

    let network_buffer = &mut [0; HOST_TO_CLIENT_MESSAGE_SIZE];
    loop {
        if let Ok(message) = message_receiver.try_recv() {
            match message {
                UIToJoinedMessage::Leave => break,
            }
        }

        let network_result = udp_socket.peek(network_buffer);
        if let Ok(bytes_amount) = network_result {
            if network_buffer.first() == Some(&2) {
                let message = udp_socket
                    .recv_large()
                    .unwrap()
                    .as_slice()
                    .try_into()
                    .unwrap();
                handle_network_message(message, &message_sender);
            } else {
                let message_result = network_buffer.as_slice().try_into();
                if let Ok(network_message) = message_result {
                    udp_socket.recv(&mut []).unwrap();
                    handle_network_message(network_message, &message_sender);
                }
            }
        }
    }
    println!("Leaving...");

    let network_buffer: Vec<u8> = ClientToHostNetworkMessage::Left(id).into();
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
        HostToClientNetworkMessage::Frame(frame) => (),
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
