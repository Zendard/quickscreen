use scap::frame::BGRxFrame;
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
        let buffer: [u8; HOST_TO_CLIENT_MESSAGE_SIZE] = message.into();
        socket.send_to(&buffer, self.address).unwrap();
    }
}

#[derive(Debug)]
pub enum ClientToHostNetworkMessage {
    JoinRequest(ClientID),
    Left(ClientID),
}
pub const CLIENT_TO_HOST_MESSAGE_SIZE: usize = 3;

impl From<ClientToHostNetworkMessage> for [u8; CLIENT_TO_HOST_MESSAGE_SIZE] {
    fn from(value: ClientToHostNetworkMessage) -> Self {
        match value {
            ClientToHostNetworkMessage::JoinRequest(id) => [1, id.0 as u8, (id.0 >> 8) as u8],
            ClientToHostNetworkMessage::Left(id) => [2, id.0 as u8, (id.0 >> 8) as u8],
        }
    }
}

#[derive(Debug)]
pub enum NetworkConversionError {
    EmptyBuffer,
    UnrecognizedSignature,
    MalformedMessage,
}

impl TryFrom<[u8; CLIENT_TO_HOST_MESSAGE_SIZE]> for ClientToHostNetworkMessage {
    type Error = NetworkConversionError;
    fn try_from(value: [u8; CLIENT_TO_HOST_MESSAGE_SIZE]) -> Result<Self, Self::Error> {
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
    Frame(BGRxFrame),
}
pub const HOST_TO_CLIENT_MESSAGE_SIZE: usize = std::mem::size_of::<BGRxFrame>() + 1;

impl From<HostToClientNetworkMessage> for [u8; HOST_TO_CLIENT_MESSAGE_SIZE] {
    fn from(value: HostToClientNetworkMessage) -> Self {
        match value {
            HostToClientNetworkMessage::JoinRequestResponse(accepted) => {
                let mut buffer = [0; HOST_TO_CLIENT_MESSAGE_SIZE];
                buffer[0] = 1;
                buffer[1] = accepted as u8;
                buffer
            }
            HostToClientNetworkMessage::Frame(frame) => {
                let mut vec = frame.data.clone();
                vec.push(2);
                let mut buffer = [0; HOST_TO_CLIENT_MESSAGE_SIZE];
                for i in 0..buffer.len() {
                    buffer[i] = *vec.get(i).unwrap_or(&0);
                }
                buffer
            }
        }
    }
}

impl TryFrom<[u8; HOST_TO_CLIENT_MESSAGE_SIZE]> for HostToClientNetworkMessage {
    type Error = NetworkConversionError;
    fn try_from(value: [u8; HOST_TO_CLIENT_MESSAGE_SIZE]) -> Result<Self, Self::Error> {
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
