use std::{error::Error, fmt::Display, str::Utf8Error};

pub mod client_msg;
pub mod game_state;
pub mod server_msg;

pub const LOBBY_ID_LEN: usize = 4;

pub type LobbyId = String;

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub enum DeserializeMessageError {
    EmptyMessage,
    InvalidBallPosition,
    InvalidByteCount,
    InvalidPaddlePosition,
    UnrecognisedMessageVariant,
    InvalidState,
    Utf8Error(Utf8Error),
}

impl Display for DeserializeMessageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeserializeMessageError::EmptyMessage => Display::fmt("empty message", f),
            DeserializeMessageError::InvalidBallPosition => {
                Display::fmt("invalid ball position", f)
            }
            DeserializeMessageError::InvalidByteCount => Display::fmt("invalid amount of bytes", f),
            DeserializeMessageError::InvalidPaddlePosition => {
                Display::fmt("invalid paddle position", f)
            }
            DeserializeMessageError::InvalidState => {
                Display::fmt("invalid state", f)
            }
            DeserializeMessageError::UnrecognisedMessageVariant => {
                Display::fmt("unrecognised message", f)
            }
            DeserializeMessageError::Utf8Error(err) => Display::fmt(err, f),
        }
    }
}

impl Error for DeserializeMessageError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            DeserializeMessageError::EmptyMessage
            | DeserializeMessageError::InvalidBallPosition
            | DeserializeMessageError::InvalidByteCount
            | DeserializeMessageError::InvalidPaddlePosition
            | DeserializeMessageError::InvalidState
            | DeserializeMessageError::UnrecognisedMessageVariant => None,
            DeserializeMessageError::Utf8Error(source) => Some(source),
        }
    }
}

fn validate_state_and_get_message_id(
    value: &[u8],
    expected_state_id: u8,
) -> Result<u8, DeserializeMessageError> {
    if value.len() == 0 {
        return Err(DeserializeMessageError::EmptyMessage);
    }
    let state_id = value[0] >> 4;
    if state_id != expected_state_id {
        return Err(DeserializeMessageError::InvalidState);
    }
    let message_id = value[0] & 0b1111;
    Ok(message_id)
}

fn validate_byte_count(slice: &[u8], exp_len: usize) -> Result<(), DeserializeMessageError> {
    if slice.len() != exp_len {
        Err(DeserializeMessageError::InvalidByteCount)
    } else {
        Ok(())
    }
}

#[cfg(test)]
#[macro_export]
macro_rules! assert_serialize {
    ($message:expr, $expected:expr $(,)?) => {
        assert_eq!(Vec::<u8>::from($message), $expected)
    };
}

#[cfg(test)]
#[macro_export]
macro_rules! assert_deserialize {
    ($type:tt, $bytes:expr, $expected:expr $(,)?) => {
        assert_eq!($type::try_from($bytes.as_slice()), $expected)
    };
}

#[cfg(test)]
#[macro_export]
macro_rules! assert_serialize_and_back {
    ($message:expr $(,)?) => {
        assert_eq!(
            Vec::<u8>::from($message.clone()).as_slice().try_into(),
            Ok($message)
        )
    };
}
