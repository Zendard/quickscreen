use crate::encoding::{
    Encoder,
    network::{
        CLIENT_TO_HOST_MESSAGE_SIZE, Client, ClientID, ClientToHostNetworkMessage,
        HostToClientNetworkMessage,
    },
};
use gstreamer::prelude::{ElementExt, GstObjectExt};
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    sync::mpsc::{Receiver, Sender},
};

pub enum HostingToUIMessage {
    JoinRequest(ClientID),
    ClientLeft(ClientID),
}

#[derive(Debug)]
pub enum UIToHostingMessage {
    Stop,
    JoinRequestResponse(ClientID, bool),
}

struct HostingState {
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
    let encoder = Encoder::new().unwrap();
    let udp_socket =
        UdpSocket::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port)).unwrap();
    let mut state = HostingState {
        udp_socket,
        pending_clients: HashMap::new(),
        accepted_clients: HashMap::new(),
        refused_clients: HashMap::new(),
    };

    let client_to_host_buffer = &mut [0; CLIENT_TO_HOST_MESSAGE_SIZE];
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

        let network_result = state.udp_socket.peek_from(client_to_host_buffer);
        if let Ok((_, origin)) = network_result {
            let message_result = client_to_host_buffer.as_slice().try_into();
            if let Ok(network_message) = message_result {
                state.udp_socket.recv(&mut []).unwrap();
                handle_network_message(network_message, origin, &message_sender, &mut state);
            }
        }

        let bus = encoder.pipeline.bus().unwrap();

        for msg in bus.iter_timed(gstreamer::ClockTime::NONE) {
            use gstreamer::MessageView;

            match msg.view() {
                MessageView::Error(err) => {
                    eprintln!(
                        "Error from {}: {}",
                        err.src().map(|s| s.path_string()).unwrap_or_default(),
                        err.error()
                    );
                    break;
                }
                MessageView::Eos(..) => break,
                _ => {}
            }
        }
    }

    encoder.pipeline.set_state(gstreamer::State::Null).unwrap();
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
        ClientToHostNetworkMessage::Left(client_id) => {
            handle_client_left(client_id, ui_sender, state)
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
        .insert(client_id, client_id.as_client(client_address));

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

fn handle_client_left(
    client_id: ClientID,
    message_sender: &Sender<HostingToUIMessage>,
    state: &mut HostingState,
) {
    println!("Client {} left", client_id.0);
    state.accepted_clients.remove(&client_id);
    message_sender
        .send(HostingToUIMessage::ClientLeft(client_id))
        .unwrap()
}
