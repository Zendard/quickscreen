use crate::host::network::ClientID;

use self::network::ClientToHostNetworkMessage;
use std::{
    collections::HashSet,
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    sync::mpsc::{Receiver, Sender},
};

mod capture;
pub mod network;

pub enum HostingToUIMessage {
    JoinRequest(ClientID),
}

#[derive(Debug)]
pub enum UIToHostingMessage {
    Stop,
    JoinRequestResponse(ClientID, bool),
}

struct HostingState {
    // capturer: Capturer,
    udp_socket: UdpSocket,
    accepted_clients: HashSet<ClientID>,
    refused_clients: HashSet<ClientID>,
}

pub fn host(
    port: u16,
    message_sender: Sender<HostingToUIMessage>,
    message_receiver: Receiver<UIToHostingMessage>,
) {
    // let capturer = capture::new().unwrap();
    let udp_socket =
        UdpSocket::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port)).unwrap();
    let mut state = HostingState {
        // capturer,
        udp_socket,
        accepted_clients: HashSet::new(),
        refused_clients: HashSet::new(),
    };

    // state.capturer.start_capture();

    let mut network_buffer = [0; std::mem::size_of::<ClientToHostNetworkMessage>() + 1];
    state.udp_socket.set_nonblocking(true).unwrap();

    loop {
        if let Ok(message) = message_receiver.try_recv() {
            match message {
                UIToHostingMessage::Stop => break,
                UIToHostingMessage::JoinRequestResponse(client_id, accepted) => {
                    if accepted {
                        println!("Client {} accepted", &client_id.0);
                        state.accepted_clients.insert(client_id);
                    } else {
                        println!("Client {} refused", &client_id.0);
                        state.refused_clients.insert(client_id);
                    }
                }
            }
        }
        // let frame = capturer.get_next_frame().unwrap();

        if let Ok(bytes_amount) = state
            .udp_socket
            .peek(&mut [0; std::mem::size_of::<ClientToHostNetworkMessage>()])
        {
            if bytes_amount >= std::mem::size_of::<ClientToHostNetworkMessage>() {
                state.udp_socket.recv(&mut network_buffer).unwrap();
                let network_message = ClientToHostNetworkMessage::try_from(network_buffer);
                if network_message.is_err() {
                    continue;
                }

                handle_network_message(network_message.unwrap(), &message_sender, &state);
            }
        }
    }

    // state.capturer.stop_capture();
    println!("Stopped hosting");
}

fn handle_network_message(
    message: ClientToHostNetworkMessage,
    ui_sender: &Sender<HostingToUIMessage>,
    state: &HostingState,
) {
    match message {
        ClientToHostNetworkMessage::JoinRequest(client_id) => {
            handle_join_request(client_id, ui_sender, state)
        }
    }
}

fn handle_join_request(
    client_id: ClientID,
    ui_sender: &Sender<HostingToUIMessage>,
    state: &HostingState,
) {
    if state.refused_clients.contains(&client_id) {
        return;
    }

    ui_sender
        .send(HostingToUIMessage::JoinRequest(client_id))
        .unwrap();
}
