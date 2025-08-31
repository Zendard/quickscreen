use crate::{
    encoding::RESOLUTION,
    encoding::{
        NetworkFrame,
        network::{
            ClientID, ClientToHostNetworkMessage, HOST_TO_CLIENT_MESSAGE_SIZE,
            HostToClientNetworkMessage, LargeSend, MAX_UDP_SEND_SIZE,
        },
    },
};
use libadwaita::gtk::cairo::{Format, ImageSurface, Surface};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    sync::mpsc::{Receiver, Sender},
};

#[derive(Debug)]
pub enum JoinedToUIMessage {
    JoinRequestResponse(bool),
    Frame(NetworkFrame),
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
        if let Ok(amount_of_bytes) = network_result {
            if amount_of_bytes >= MAX_UDP_SEND_SIZE - 1 {
                println!("Receiving frame...");
                let message = udp_socket
                    .recv_large()
                    .unwrap()
                    .as_slice()
                    .try_into()
                    .unwrap();
                handle_network_message(message, &message_sender);
            } else {
                println!("Receiving something else...");
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
        HostToClientNetworkMessage::Frame(frame) => handle_frame(frame, message_sender),
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

fn handle_frame(frame: NetworkFrame, message_sender: &Sender<JoinedToUIMessage>) {
    message_sender
        .send(JoinedToUIMessage::Frame(frame))
        .unwrap()
}

impl From<NetworkFrame> for Surface {
    fn from(value: NetworkFrame) -> Self {
        println!("Converting NetworkFrame to Surface...");
        let mut rgbx_data = Vec::with_capacity(RESOLUTION.0 * RESOLUTION.1 * 4);
        for i in 0..value.data.len() {
            rgbx_data.push(value.data[i]);
            if (i + 1) % 3 == 0 {
                rgbx_data.push(0);
            }
        }
        dbg!(rgbx_data.len());

        ImageSurface::create_for_data(
            rgbx_data,
            libadwaita::gtk::cairo::Format::Rgb24,
            RESOLUTION.0 as i32,
            RESOLUTION.1 as i32,
            Format::Rgb24.stride_for_width(RESOLUTION.0 as u32).unwrap(),
        )
        .unwrap()
        .create_similar(
            libadwaita::gtk::cairo::Content::Color,
            RESOLUTION.0 as i32,
            RESOLUTION.1 as i32,
        )
        .unwrap()
    }
}
