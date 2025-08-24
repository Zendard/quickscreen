use self::network::ClientToHostNetworkMessage;
use crate::host::network::{Client, ClientID, HostToClientNetworkMessage};
use std::{
    collections::HashMap,
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
    pending_clients: HashMap<ClientID, Client>,
    accepted_clients: HashMap<ClientID, Client>,
    refused_clients: HashMap<ClientID, Client>,
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
        pending_clients: HashMap::new(),
        accepted_clients: HashMap::new(),
        refused_clients: HashMap::new(),
    };

    // state.capturer.start_capture();

    let mut network_buffer = [0; std::mem::size_of::<ClientToHostNetworkMessage>() + 1];
    state.udp_socket.set_nonblocking(true).unwrap();

    loop {
        if let Ok(message) = message_receiver.try_recv() {
            match message {
                UIToHostingMessage::Stop => break,
                UIToHostingMessage::JoinRequestResponse(client_id, accepted) => {
                    handle_join_request_response(client_id, accepted, &mut state)
                }
            }
        }
        // let frame = capturer.get_next_frame().unwrap();

        if let Ok(bytes_amount) = state
            .udp_socket
            .peek(&mut [0; std::mem::size_of::<ClientToHostNetworkMessage>()])
        {
            if bytes_amount >= std::mem::size_of::<ClientToHostNetworkMessage>() {
                let (_, origin) = state.udp_socket.recv_from(&mut network_buffer).unwrap();
                let network_message = ClientToHostNetworkMessage::try_from(network_buffer);
                if network_message.is_err() {
                    continue;
                }

                handle_network_message(
                    network_message.unwrap(),
                    origin,
                    &message_sender,
                    &mut state,
                );
            }
        }
    }

    // state.capturer.stop_capture();
    println!("Stopped hosting");
}

fn handle_network_message(
    message: ClientToHostNetworkMessage,
    origin: SocketAddr,
    ui_sender: &Sender<HostingToUIMessage>,
    state: &mut HostingState,
) {
    match message {
        ClientToHostNetworkMessage::JoinRequest(client_id) => {
            handle_join_request(client_id, origin, ui_sender, state)
        }
    }
}

fn handle_join_request(
    client_id: ClientID,
    client_address: SocketAddr,
    ui_sender: &Sender<HostingToUIMessage>,
    state: &mut HostingState,
) {
    if state.refused_clients.contains_key(&client_id) {
        return;
    }

    state
        .pending_clients
        .insert(client_id, client_id.to_client(client_address));

    ui_sender
        .send(HostingToUIMessage::JoinRequest(client_id))
        .unwrap();
}

fn handle_join_request_response(client_id: ClientID, accepted: bool, state: &mut HostingState) {
    let client = state.pending_clients.get(&client_id).unwrap().clone();
    if accepted {
        println!("Client {} accepted", &client_id.0);
        state.accepted_clients.insert(client_id, client.clone());
    } else {
        println!("Client {} refused", &client_id.0);
        state.refused_clients.insert(client_id, client.clone());
    }
    client.send_message(
        &state.udp_socket,
        HostToClientNetworkMessage::JoinRequestResponse(accepted),
    );
}
