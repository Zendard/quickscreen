#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
pub struct ClientID(pub u16);

impl ClientID {
    pub fn generate() -> Self {
        let id: u16 = rand::random();
        Self(id)
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

impl TryFrom<[u8; 3]> for ClientToHostNetworkMessage {
    type Error = NetworkConversionError;
    fn try_from(value: [u8; 3]) -> Result<Self, Self::Error> {
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
