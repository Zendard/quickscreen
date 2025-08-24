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
    address: SocketAddr,
}

impl Client {
    pub fn send_message(&self, socket: &UdpSocket, message: HostToClientNetworkMessage) {
        let buffer: [u8; std::mem::size_of::<HostToClientNetworkMessage>() + 1] = message.into();
        socket.send_to(&buffer, self.address).unwrap();
    }
}

#[derive(Debug)]
pub enum ClientToHostNetworkMessage {
    JoinRequest(ClientID),
}

impl From<ClientToHostNetworkMessage>
    for [u8; std::mem::size_of::<ClientToHostNetworkMessage>() + 1]
{
    fn from(value: ClientToHostNetworkMessage) -> Self {
        match value {
            ClientToHostNetworkMessage::JoinRequest(id) => [1, id.0 as u8, (id.0 >> 8) as u8],
        }
    }
}

#[derive(Debug)]
pub enum NetworkConversionError {
    EmptyBuffer,
    UnrecognizedSignature,
    MalformedMessage,
}

impl TryFrom<[u8; std::mem::size_of::<ClientToHostNetworkMessage>() + 1]>
    for ClientToHostNetworkMessage
{
    type Error = NetworkConversionError;
    fn try_from(
        value: [u8; std::mem::size_of::<ClientToHostNetworkMessage>() + 1],
    ) -> Result<Self, Self::Error> {
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
            _ => Err(NetworkConversionError::UnrecognizedSignature),
        }
    }
}

#[derive(Debug)]
pub enum HostToClientNetworkMessage {
    JoinRequestResponse(bool),
}

impl From<HostToClientNetworkMessage>
    for [u8; std::mem::size_of::<HostToClientNetworkMessage>() + 1]
{
    fn from(value: HostToClientNetworkMessage) -> Self {
        match value {
            HostToClientNetworkMessage::JoinRequestResponse(accepted) => [0, accepted as u8],
        }
    }
}

impl TryFrom<[u8; std::mem::size_of::<HostToClientNetworkMessage>() + 1]>
    for HostToClientNetworkMessage
{
    type Error = NetworkConversionError;
    fn try_from(
        value: [u8; std::mem::size_of::<HostToClientNetworkMessage>() + 1],
    ) -> Result<Self, Self::Error> {
        let first_byte = value.first().ok_or(NetworkConversionError::EmptyBuffer)?;
        match first_byte {
            0 => {
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
