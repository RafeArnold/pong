use super::{
    validate_byte_count, validate_state_and_get_message_id, DeserializeMessageError, LOBBY_ID_LEN,
};

/// the largest number of bytes a serialized client message could take up.
/// [`AwaitingOpenClientMessage::JoinLobby`] is the largest client message when serialized (one byte for the identifier + lobby id length).
pub const MAX_CLIENT_MESSAGE_SIZE: usize = 1 + LOBBY_ID_LEN;

#[cfg_attr(test, derive(Clone, Debug, PartialEq))]
pub enum AwaitingOpenClientMessage<'a> {
    NewLobby,
    JoinLobby { lobby_id: &'a str },
}

#[cfg_attr(test, derive(Clone, Debug, PartialEq))]
pub enum AwaitingReadyClientMessage {
    Ready,
    Unready,
}

#[cfg_attr(test, derive(Clone, Debug, PartialEq))]
pub enum PlayingClientMessage {
    MovePaddle { pos: u8 },
}

impl From<AwaitingOpenClientMessage<'_>> for Vec<u8> {
    fn from(value: AwaitingOpenClientMessage) -> Self {
        match value {
            AwaitingOpenClientMessage::NewLobby => vec![0],
            AwaitingOpenClientMessage::JoinLobby { lobby_id } => {
                [&[1], lobby_id.as_bytes()].concat()
            }
        }
    }
}

impl<'a> TryFrom<&'a [u8]> for AwaitingOpenClientMessage<'a> {
    type Error = DeserializeMessageError;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        match validate_state_and_get_message_id(value, 0)? {
            0 => {
                validate_byte_count(value, 1)?;
                Ok(Self::NewLobby)
            }
            1 => {
                validate_byte_count(value, LOBBY_ID_LEN + 1)?;
                let lobby_id = std::str::from_utf8(&value[1..])
                    .map_err(|err| DeserializeMessageError::Utf8Error(err))?;
                Ok(Self::JoinLobby { lobby_id })
            }
            _ => Err(DeserializeMessageError::UnrecognisedMessageVariant),
        }
    }
}

impl From<AwaitingReadyClientMessage> for Vec<u8> {
    fn from(value: AwaitingReadyClientMessage) -> Self {
        let mut bytes = match value {
            AwaitingReadyClientMessage::Ready => vec![0],
            AwaitingReadyClientMessage::Unready => vec![1],
        };
        bytes[0] |= 1 << 4;
        bytes
    }
}

impl TryFrom<&[u8]> for AwaitingReadyClientMessage {
    type Error = DeserializeMessageError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        match validate_state_and_get_message_id(value, 1)? {
            0 => {
                validate_byte_count(value, 1)?;
                Ok(Self::Ready)
            }
            1 => {
                validate_byte_count(value, 1)?;
                Ok(Self::Unready)
            }
            _ => Err(DeserializeMessageError::UnrecognisedMessageVariant),
        }
    }
}

impl From<PlayingClientMessage> for Vec<u8> {
    fn from(value: PlayingClientMessage) -> Self {
        let mut bytes = match value {
            PlayingClientMessage::MovePaddle { pos } => vec![0, pos],
        };
        bytes[0] |= 2 << 4;
        bytes
    }
}

impl TryFrom<&[u8]> for PlayingClientMessage {
    type Error = DeserializeMessageError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        match validate_state_and_get_message_id(value, 2)? {
            0 => {
                validate_byte_count(value, 2)?;
                Ok(Self::MovePaddle { pos: value[1] })
            }
            _ => Err(DeserializeMessageError::UnrecognisedMessageVariant),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        assert_deserialize, assert_serialize, assert_serialize_and_back,
        client_msg::{
            AwaitingOpenClientMessage, AwaitingReadyClientMessage, DeserializeMessageError,
            PlayingClientMessage,
        },
    };

    #[test]
    fn awaiting_open_serialize() {
        assert_serialize!(AwaitingOpenClientMessage::NewLobby, vec![0]);
        let lobby_id = "F7BW";
        assert_serialize!(
            AwaitingOpenClientMessage::JoinLobby { lobby_id },
            [&[1], lobby_id.as_bytes()].concat(),
        );
    }

    #[test]
    fn awaiting_open_deserialize_ok() {
        assert_deserialize!(
            AwaitingOpenClientMessage,
            [0],
            Ok(AwaitingOpenClientMessage::NewLobby),
        );
        let lobby_id = "A5EZ";
        assert_deserialize!(
            AwaitingOpenClientMessage,
            [&[1], lobby_id.as_bytes()].concat(),
            Ok(AwaitingOpenClientMessage::JoinLobby { lobby_id }),
        );
    }

    #[test]
    fn awaiting_open_deserialize_err() {
        // empty message.
        assert_deserialize!(
            AwaitingOpenClientMessage,
            [],
            Err(DeserializeMessageError::EmptyMessage),
        );
        // new lobby message with extra bytes.
        assert_deserialize!(
            AwaitingOpenClientMessage,
            [&[0], "A5EZ".as_bytes()].concat(),
            Err(DeserializeMessageError::InvalidByteCount),
        );
        // join lobby message with no lobby id bytes.
        assert_deserialize!(
            AwaitingOpenClientMessage,
            [1],
            Err(DeserializeMessageError::InvalidByteCount),
        );
        // join lobby message with not enough bytes.
        assert_deserialize!(
            AwaitingOpenClientMessage,
            [&[1], "A5E".as_bytes()].concat(),
            Err(DeserializeMessageError::InvalidByteCount),
        );
        // join lobby message with too many bytes.
        assert_deserialize!(
            AwaitingOpenClientMessage,
            [&[1], "A5EZ8".as_bytes()].concat(),
            Err(DeserializeMessageError::InvalidByteCount),
        );
        // join lobby message with invalid utf-8.
        assert!(matches!(
            AwaitingOpenClientMessage::try_from([1, 255, 255, 255, 255].as_slice()),
            Err(DeserializeMessageError::Utf8Error(_))
        ));
        // invalid state variant.
        assert_deserialize!(
            AwaitingOpenClientMessage,
            [1 << 4],
            Err(DeserializeMessageError::InvalidState),
        );
        // unrecognised message variant.
        assert_deserialize!(
            AwaitingOpenClientMessage,
            [2],
            Err(DeserializeMessageError::UnrecognisedMessageVariant),
        );
    }

    #[test]
    fn awaiting_ready_serialize() {
        assert_serialize!(
            Vec::<u8>::from(AwaitingReadyClientMessage::Ready),
            vec![1 << 4]
        );
        assert_serialize!(
            Vec::<u8>::from(AwaitingReadyClientMessage::Unready),
            vec![1 << 4 | 1]
        );
    }

    #[test]
    fn awaiting_ready_deserialize_ok() {
        assert_deserialize!(
            AwaitingReadyClientMessage,
            [1 << 4],
            Ok(AwaitingReadyClientMessage::Ready),
        );
        assert_deserialize!(
            AwaitingReadyClientMessage,
            [1 << 4 | 1],
            Ok(AwaitingReadyClientMessage::Unready),
        );
    }

    #[test]
    fn awaiting_ready_deserialize_err() {
        // empty message.
        assert_deserialize!(
            AwaitingReadyClientMessage,
            [],
            Err(DeserializeMessageError::EmptyMessage),
        );
        // ready message with extra bytes.
        assert_deserialize!(
            AwaitingReadyClientMessage,
            [1 << 4, 0],
            Err(DeserializeMessageError::InvalidByteCount),
        );
        // unready message with extra bytes.
        assert_deserialize!(
            AwaitingReadyClientMessage,
            [1 << 4 | 1, 0],
            Err(DeserializeMessageError::InvalidByteCount),
        );
        // invalid state variant.
        assert_deserialize!(
            AwaitingReadyClientMessage,
            [0],
            Err(DeserializeMessageError::InvalidState),
        );
        // unrecognised message variant.
        assert_deserialize!(
            AwaitingReadyClientMessage,
            [1 << 4 | 2],
            Err(DeserializeMessageError::UnrecognisedMessageVariant),
        );
    }

    #[test]
    fn playing_serialize() {
        let pos = 6;
        assert_serialize!(PlayingClientMessage::MovePaddle { pos }, vec![2 << 4, pos]);
        let pos = 154;
        assert_serialize!(PlayingClientMessage::MovePaddle { pos }, vec![2 << 4, pos]);
    }

    #[test]
    fn playing_deserialize_ok() {
        assert_deserialize!(
            PlayingClientMessage,
            [2 << 4, 5],
            Ok(PlayingClientMessage::MovePaddle { pos: 5 }),
        );
        assert_deserialize!(
            PlayingClientMessage,
            [2 << 4, 76],
            Ok(PlayingClientMessage::MovePaddle { pos: 76 }),
        );
    }

    #[test]
    fn playing_deserialize_err() {
        // empty message.
        assert_deserialize!(
            PlayingClientMessage,
            [],
            Err(DeserializeMessageError::EmptyMessage),
        );
        // move paddle message with missing byte.
        assert_deserialize!(
            PlayingClientMessage,
            [2 << 4],
            Err(DeserializeMessageError::InvalidByteCount),
        );
        // move paddle message with extra bytes.
        assert_deserialize!(
            PlayingClientMessage,
            [2 << 4, 5, 5],
            Err(DeserializeMessageError::InvalidByteCount),
        );
        // invalid state variant.
        assert_deserialize!(
            PlayingClientMessage,
            [0],
            Err(DeserializeMessageError::InvalidState),
        );
        // unrecognised message variant.
        assert_deserialize!(
            PlayingClientMessage,
            [2 << 4 | 1],
            Err(DeserializeMessageError::UnrecognisedMessageVariant),
        );
    }

    #[test]
    fn serialize_and_back() {
        assert_serialize_and_back!(AwaitingOpenClientMessage::NewLobby);
        assert_serialize_and_back!(AwaitingOpenClientMessage::JoinLobby { lobby_id: "AOP4" });
        assert_serialize_and_back!(AwaitingReadyClientMessage::Ready);
        assert_serialize_and_back!(AwaitingReadyClientMessage::Unready);
        assert_serialize_and_back!(PlayingClientMessage::MovePaddle { pos: 42 });
    }
}
