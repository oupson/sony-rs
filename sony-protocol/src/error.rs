use std::{array::TryFromSliceError, fmt::Display};
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TryFromPacketError {
    ProtocolError(Error),
    NotImplemented { seqnum: u8, what: &'static str },
}

impl From<Error> for TryFromPacketError {
    fn from(value: Error) -> Self {
        Self::ProtocolError(value)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Error {
    UnknownPacket(&'static str),
    PacketPending,
    InvalidValueForEnum { what: &'static str, value: u8 },
    UnknownPayloadType(u8),
    MissingBytes,
    NotImplemented(&'static str),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownPacket(name) => write!(f, "unknown packet (type = \"{}\")", name),
            Self::PacketPending => write!(f, "already in sending state"),
            Self::InvalidValueForEnum { what, value } => {
                write!(f, "invalid value for {} : {:02x}", what, value)
            }
            Self::UnknownPayloadType(t) => write!(f, "unknown payload type : {:02x?}", t),
            Self::MissingBytes => write!(f, "missing data to parse packet"),
            Self::NotImplemented(what) => write!(f, "{} is not implemented", what),
        }
    }
}

impl From<TryFromSliceError> for Error {
    fn from(_: TryFromSliceError) -> Self {
        Self::MissingBytes
    }
}

impl std::error::Error for Error {}

impl From<&Error> for Error {
    fn from(value: &Error) -> Self {
        value.clone()
    }
}

pub type Result<T> = std::result::Result<T, Error>;
