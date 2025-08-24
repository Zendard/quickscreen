use crate::host::capture::NetworkFrame;
use std::{
    hash::Hash,
    net::{SocketAddr, UdpSocket},
};

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
pub const HOST_TO_CLIENT_MESSAGE_SIZE: usize = 1920 * 1080 * 3 + 1;

impl From<HostToClientNetworkMessage> for Vec<u8> {
    fn from(value: HostToClientNetworkMessage) -> Self {
        match value {
            HostToClientNetworkMessage::JoinRequestResponse(accepted) => vec![0, (accepted as u8)],
            HostToClientNetworkMessage::Frame(frame) => {
                frame.data.iter().flat_map(|i| [i.0, i.1, i.2]).collect()
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
                dbg!(accepted);
                Ok(Self::JoinRequestResponse(accepted != 0))
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
}

const MAX_UDP_SEND_SIZE: usize = 65507;
impl LargeSend for UdpSocket {
    fn send_to_large(
        &self,
        bytes: &[u8],
        address: SocketAddr,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let amount_of_slices = bytes.len() / MAX_UDP_SEND_SIZE;

        let mut remaining_slice = bytes;

        for _ in 0..amount_of_slices {
            let (left_slice, right_slice) = remaining_slice.split_at(MAX_UDP_SEND_SIZE - 1);
            self.send_to(left_slice, address)?;
            remaining_slice = right_slice;
        }
        self.send_to(remaining_slice, address)?;

        Ok(())
    }
}
