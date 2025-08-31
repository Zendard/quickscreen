use std::{
    hash::Hash,
    net::{SocketAddr, UdpSocket},
};

use crate::encoding::NetworkFrame;

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
pub struct ClientID(pub u16);

impl ClientID {
    pub fn generate() -> Self {
        let id: u16 = rand::random();
        Self(id)
    }

    pub fn as_client(&self, address: SocketAddr) -> Client {
        Client { id: *self, address }
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Client {
    pub id: ClientID,
    pub address: SocketAddr,
}

impl Client {
    pub fn send_message(&self, socket: &UdpSocket, message: HostToClientNetworkMessage) {
        let buffer: Vec<u8> = message.into();
        socket.send_to(&buffer, self.address).unwrap();
    }
}

#[derive(Debug)]
pub enum ClientToHostNetworkMessage {
    JoinRequest(ClientID),
    Left(ClientID),
}
pub const CLIENT_TO_HOST_MESSAGE_SIZE: usize = 3;

impl From<ClientToHostNetworkMessage> for Vec<u8> {
    fn from(value: ClientToHostNetworkMessage) -> Self {
        match value {
            ClientToHostNetworkMessage::JoinRequest(id) => vec![1, id.0 as u8, (id.0 >> 8) as u8],
            ClientToHostNetworkMessage::Left(id) => vec![2, (id.0 as u8), ((id.0 >> 8) as u8)],
        }
    }
}

#[derive(Debug)]
pub enum NetworkConversionError {
    EmptyBuffer,
    UnrecognizedSignature,
    MalformedMessage,
}

impl TryFrom<&[u8]> for ClientToHostNetworkMessage {
    type Error = NetworkConversionError;
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let first_byte = value.first().ok_or(NetworkConversionError::EmptyBuffer)?;
        match first_byte {
            1 => {
                let id = *value
                    .get(1)
                    .ok_or(NetworkConversionError::MalformedMessage)?
                    as u16
                    | (*value
                        .get(2)
                        .ok_or(NetworkConversionError::MalformedMessage)?
                        as u16)
                        << 8;
                Ok(Self::JoinRequest(ClientID(id)))
            }
            2 => {
                let id = *value
                    .get(1)
                    .ok_or(NetworkConversionError::MalformedMessage)?
                    as u16
                    | (*value
                        .get(2)
                        .ok_or(NetworkConversionError::MalformedMessage)?
                        as u16)
                        << 8;
                Ok(Self::Left(ClientID(id)))
            }
            _ => Err(NetworkConversionError::UnrecognizedSignature),
        }
    }
}

#[derive(Debug)]
pub enum HostToClientNetworkMessage {
    JoinRequestResponse(bool),
    Frame(NetworkFrame),
}
pub const HOST_TO_CLIENT_MESSAGE_SIZE: usize = MAX_UDP_SEND_SIZE;

impl From<HostToClientNetworkMessage> for Vec<u8> {
    fn from(value: HostToClientNetworkMessage) -> Self {
        match value {
            HostToClientNetworkMessage::JoinRequestResponse(accepted) => vec![1, (accepted as u8)],
            HostToClientNetworkMessage::Frame(mut frame) => {
                let mut output = Vec::with_capacity(frame.data.len() + 2);
                output.push(2);
                let amount_of_sends = ((frame.data.len() + 2) / MAX_UDP_SEND_SIZE) + 1;
                output.push(amount_of_sends.try_into().unwrap());
                output.append(&mut frame.data);
                output
            }
        }
    }
}

impl TryFrom<&[u8]> for HostToClientNetworkMessage {
    type Error = NetworkConversionError;
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let first_byte = value.first().ok_or(NetworkConversionError::EmptyBuffer)?;
        match first_byte {
            1 => {
                let accepted = *value
                    .get(1)
                    .ok_or(NetworkConversionError::MalformedMessage)?;
                Ok(Self::JoinRequestResponse(accepted != 0))
            }
            2 => {
                let mut value = value;
                value.split_off_first();
                value.split_off_first();
                Ok(Self::Frame(NetworkFrame {
                    data: value.to_vec(),
                }))
            }
            _ => Err(NetworkConversionError::UnrecognizedSignature),
        }
    }
}

pub trait LargeSend {
    fn send_to_large(
        &self,
        bytes: &[u8],
        address: SocketAddr,
    ) -> Result<(), Box<dyn std::error::Error>>;

    fn recv_large(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>>;
}

pub const MAX_UDP_SEND_SIZE: usize = 65507;
impl LargeSend for UdpSocket {
    fn send_to_large(
        &self,
        bytes: &[u8],
        address: SocketAddr,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let amount_of_slices = bytes.len() / MAX_UDP_SEND_SIZE;
        dbg!(&amount_of_slices);

        let mut remaining_slice = bytes;

        for _ in 0..amount_of_slices {
            let (left_slice, right_slice) = remaining_slice.split_at(MAX_UDP_SEND_SIZE - 1);
            println!("Sending {} bytes of data...", left_slice.len());
            self.send_to(left_slice, address)?;
            remaining_slice = right_slice;
        }
        self.send_to(remaining_slice, address)?;

        Ok(())
    }
    fn recv_large(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let network_buffer = &mut [0; MAX_UDP_SEND_SIZE];
        let mut bytes_amount = 0;
        while bytes_amount == 0 {
            bytes_amount = self.recv(network_buffer).unwrap_or(0);
        }
        let mut remaining_parts = network_buffer[1] - 1;
        dbg!(remaining_parts);

        let mut output = Vec::with_capacity(MAX_UDP_SEND_SIZE * remaining_parts as usize);
        output.append(&mut network_buffer.to_vec());

        while remaining_parts > 1 {
            let bytes_amount = self.recv(network_buffer).unwrap_or(0);
            if bytes_amount == 0 {
                continue;
            }
            println!("Received {} bytes of data", bytes_amount);

            output.append(&mut network_buffer.to_vec());
            remaining_parts -= 1;
        }
        dbg!(output.len() / MAX_UDP_SEND_SIZE);
        Ok(output)
    }
}
