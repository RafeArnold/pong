use crate::game_state::{GAME_HEIGHT, GAME_WIDTH, PADDLE_HEIGHT};

use super::{
    game_state::{Ball, GameState},
    validate_byte_count, validate_state_and_get_message_id, DeserializeMessageError, LOBBY_ID_LEN,
};

const _CHECKS: () = {
    assert!(
        GAME_HEIGHT < 2u8.pow(7) - 1,
        "height of the game window is too large to serialize the ball's vertical position and direction using a single u8"
    );
    assert!(
        GAME_WIDTH < 2u8.pow(7) - 1,
        "width of the game window is too large to serialize the ball's horizontal position and direction using a single u8"
    );
    assert!(
        GAME_HEIGHT - PADDLE_HEIGHT < 2u8.pow(4) - 1,
        "height of the game window is too large to serialize both paddle positions using a single u8"
    );
};

/// the largest number of bytes a serialized server message could take up.
/// [`AwaitingNewLobbyServerMessage::NewLobbyCreated`] is the largest server message when serialized (one byte for the identifier + lobby id length).
pub const MAX_SERVER_MESSAGE_SIZE: usize = 1 + LOBBY_ID_LEN;

/// this byte is appended to the end of every server message to indicate termination.
/// we must therefore ensure that no other bytes in a message must serialize to this value.
pub const SERVER_MESSAGE_DELIMITER: u8 = u8::MAX;

#[cfg_attr(test, derive(Clone, Debug, PartialEq))]
pub enum AwaitingNewLobbyServerMessage<'a> {
    NewLobbyCreated { lobby_id: &'a str },
}

#[cfg_attr(test, derive(Clone, Debug, PartialEq))]
pub enum AwaitingJoinLobbyServerMessage {
    JoinedLobby,
    LobbyFull,
    LobbyNotFound,
}

#[cfg_attr(test, derive(Clone, Debug, PartialEq))]
pub enum AwaitingOpponentJoinServerMessage {
    OpponentJoined,
}

#[cfg_attr(test, derive(Clone, Debug, PartialEq))]
pub enum AwaitingReadyServerMessage {
    OpponentLeft,
    OpponentReadied,
    OpponentUnreadied,
    YouReadied,
    YouUnreadied,
    GameStarted,
}

#[derive(Clone)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub enum PlayingServerMessage {
    OpponentLeft,
    OpponentWon,
    YouWon,
    GameStateUpdated { game_state: GameState },
}

impl From<AwaitingNewLobbyServerMessage<'_>> for Vec<u8> {
    fn from(value: AwaitingNewLobbyServerMessage) -> Self {
        match value {
            AwaitingNewLobbyServerMessage::NewLobbyCreated { lobby_id } => {
                [&[0], lobby_id.as_bytes()].concat()
            }
        }
    }
}

impl<'a> TryFrom<&'a [u8]> for AwaitingNewLobbyServerMessage<'a> {
    type Error = DeserializeMessageError;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        match validate_state_and_get_message_id(value, 0)? {
            0 => {
                validate_byte_count(value, 1 + LOBBY_ID_LEN)?;
                let lobby_id = std::str::from_utf8(&value[1..])
                    .map_err(|err| DeserializeMessageError::Utf8Error(err))?;
                Ok(AwaitingNewLobbyServerMessage::NewLobbyCreated { lobby_id })
            }
            _ => Err(DeserializeMessageError::UnrecognisedMessageVariant),
        }
    }
}

impl From<AwaitingJoinLobbyServerMessage> for Vec<u8> {
    fn from(value: AwaitingJoinLobbyServerMessage) -> Self {
        let mut bytes = match value {
            AwaitingJoinLobbyServerMessage::JoinedLobby => vec![0],
            AwaitingJoinLobbyServerMessage::LobbyFull => vec![1],
            AwaitingJoinLobbyServerMessage::LobbyNotFound => vec![2],
        };
        bytes[0] |= 1 << 4;
        bytes
    }
}

impl TryFrom<&[u8]> for AwaitingJoinLobbyServerMessage {
    type Error = DeserializeMessageError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        match validate_state_and_get_message_id(value, 1)? {
            0 => {
                validate_byte_count(value, 1)?;
                Ok(AwaitingJoinLobbyServerMessage::JoinedLobby)
            }
            1 => {
                validate_byte_count(value, 1)?;
                Ok(AwaitingJoinLobbyServerMessage::LobbyFull)
            }
            2 => {
                validate_byte_count(value, 1)?;
                Ok(AwaitingJoinLobbyServerMessage::LobbyNotFound)
            }
            _ => Err(DeserializeMessageError::UnrecognisedMessageVariant),
        }
    }
}

impl From<AwaitingOpponentJoinServerMessage> for Vec<u8> {
    fn from(value: AwaitingOpponentJoinServerMessage) -> Self {
        let mut bytes = match value {
            AwaitingOpponentJoinServerMessage::OpponentJoined => vec![0],
        };
        bytes[0] |= 2 << 4;
        bytes
    }
}

impl TryFrom<&[u8]> for AwaitingOpponentJoinServerMessage {
    type Error = DeserializeMessageError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        match validate_state_and_get_message_id(value, 2)? {
            0 => {
                validate_byte_count(value, 1)?;
                Ok(AwaitingOpponentJoinServerMessage::OpponentJoined)
            }
            _ => Err(DeserializeMessageError::UnrecognisedMessageVariant),
        }
    }
}

impl From<AwaitingReadyServerMessage> for Vec<u8> {
    fn from(value: AwaitingReadyServerMessage) -> Self {
        let mut bytes = match value {
            AwaitingReadyServerMessage::OpponentLeft => vec![0],
            AwaitingReadyServerMessage::OpponentReadied => vec![1],
            AwaitingReadyServerMessage::OpponentUnreadied => vec![2],
            AwaitingReadyServerMessage::YouReadied => vec![3],
            AwaitingReadyServerMessage::YouUnreadied => vec![4],
            AwaitingReadyServerMessage::GameStarted => vec![5],
        };
        bytes[0] |= 3 << 4;
        bytes
    }
}

impl TryFrom<&[u8]> for AwaitingReadyServerMessage {
    type Error = DeserializeMessageError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        match validate_state_and_get_message_id(value, 3)? {
            0 => {
                validate_byte_count(value, 1)?;
                Ok(AwaitingReadyServerMessage::OpponentLeft)
            }
            1 => {
                validate_byte_count(value, 1)?;
                Ok(AwaitingReadyServerMessage::OpponentReadied)
            }
            2 => {
                validate_byte_count(value, 1)?;
                Ok(AwaitingReadyServerMessage::OpponentUnreadied)
            }
            3 => {
                validate_byte_count(value, 1)?;
                Ok(AwaitingReadyServerMessage::YouReadied)
            }
            4 => {
                validate_byte_count(value, 1)?;
                Ok(AwaitingReadyServerMessage::YouUnreadied)
            }
            5 => {
                validate_byte_count(value, 1)?;
                Ok(AwaitingReadyServerMessage::GameStarted)
            }
            _ => Err(DeserializeMessageError::UnrecognisedMessageVariant),
        }
    }
}

impl From<PlayingServerMessage> for Vec<u8> {
    fn from(value: PlayingServerMessage) -> Self {
        let mut bytes = match value {
            PlayingServerMessage::OpponentLeft => vec![0],
            PlayingServerMessage::OpponentWon => vec![1],
            PlayingServerMessage::YouWon => vec![2],
            PlayingServerMessage::GameStateUpdated { game_state } => vec![
                3,
                // serialize the position of both paddles into a single byte.
                // an assertion is performed at the top of the file to ensure this is possible without loss of information.
                game_state.left_paddle << 4 | (game_state.right_paddle & 0b1111),
                // for each axis, serialize the position and direction of the ball into a single byte.
                // an assertion is performed at the top of the file to ensure this is possible without loss of information.
                game_state.ball.x << 1 | game_state.ball.moving_right as u8,
                game_state.ball.y << 1 | game_state.ball.moving_down as u8,
            ],
        };
        bytes[0] |= 4 << 4;
        bytes
    }
}

impl TryFrom<&[u8]> for PlayingServerMessage {
    type Error = DeserializeMessageError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        match validate_state_and_get_message_id(value, 4)? {
            0 => {
                validate_byte_count(value, 1)?;
                Ok(PlayingServerMessage::OpponentLeft)
            }
            1 => {
                validate_byte_count(value, 1)?;
                Ok(PlayingServerMessage::OpponentWon)
            }
            2 => {
                validate_byte_count(value, 1)?;
                Ok(PlayingServerMessage::YouWon)
            }
            3 => {
                validate_byte_count(value, 4)?;
                let left_paddle = value[1] >> 4;
                if left_paddle > GAME_HEIGHT - PADDLE_HEIGHT {
                    return Err(DeserializeMessageError::InvalidPaddlePosition);
                }
                let right_paddle = value[1] & 0b1111;
                if right_paddle > GAME_HEIGHT - PADDLE_HEIGHT {
                    return Err(DeserializeMessageError::InvalidPaddlePosition);
                }
                let x = value[2] >> 1;
                let y = value[3] >> 1;
                if x >= GAME_WIDTH || y >= GAME_HEIGHT {
                    return Err(DeserializeMessageError::InvalidBallPosition);
                }
                Ok(PlayingServerMessage::GameStateUpdated {
                    game_state: GameState {
                        left_paddle,
                        right_paddle,
                        ball: Ball {
                            x,
                            y,
                            moving_right: value[2] & 1 == 1,
                            moving_down: value[3] & 1 == 1,
                        },
                    },
                })
            }
            _ => Err(DeserializeMessageError::UnrecognisedMessageVariant),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        assert_deserialize, assert_serialize, assert_serialize_and_back,
        game_state::{Ball, GameState},
        server_msg::{
            AwaitingJoinLobbyServerMessage, AwaitingNewLobbyServerMessage,
            AwaitingOpponentJoinServerMessage, AwaitingReadyServerMessage, PlayingServerMessage,
        },
        DeserializeMessageError,
    };

    #[test]
    fn awaiting_new_lobby_serialize() {
        let lobby_id = "A5EZ";
        assert_serialize!(
            AwaitingNewLobbyServerMessage::NewLobbyCreated { lobby_id },
            [&[0], lobby_id.as_bytes()].concat()
        );
    }

    #[test]
    fn awaiting_new_lobby_deserialize_ok() {
        let lobby_id = "F7BW";
        assert_deserialize!(
            AwaitingNewLobbyServerMessage,
            [&[0], lobby_id.as_bytes()].concat(),
            Ok(AwaitingNewLobbyServerMessage::NewLobbyCreated { lobby_id }),
        );
    }

    #[test]
    fn awaiting_new_lobby_deserialize_err() {
        // empty message.
        assert_deserialize!(
            AwaitingNewLobbyServerMessage,
            [],
            Err(DeserializeMessageError::EmptyMessage),
        );
        // new lobby created message with no lobby id bytes.
        assert_deserialize!(
            AwaitingNewLobbyServerMessage,
            [0],
            Err(DeserializeMessageError::InvalidByteCount),
        );
        // new lobby created message with not enough bytes.
        assert_deserialize!(
            AwaitingNewLobbyServerMessage,
            [&[0], "A5E".as_bytes()].concat(),
            Err(DeserializeMessageError::InvalidByteCount),
        );
        // new lobby created message with too many bytes.
        assert_deserialize!(
            AwaitingNewLobbyServerMessage,
            [&[0], "A5EZ8".as_bytes()].concat(),
            Err(DeserializeMessageError::InvalidByteCount),
        );
        // new lobby created with invalid utf-8.
        assert!(matches!(
            AwaitingNewLobbyServerMessage::try_from([0, 255, 255, 255, 255].as_slice()),
            Err(DeserializeMessageError::Utf8Error(_))
        ));
        // invalid state variant.
        assert_deserialize!(
            AwaitingNewLobbyServerMessage,
            [1 << 4],
            Err(DeserializeMessageError::InvalidState),
        );
        // unrecognised message variant.
        assert_deserialize!(
            AwaitingNewLobbyServerMessage,
            [1],
            Err(DeserializeMessageError::UnrecognisedMessageVariant),
        );
    }

    #[test]
    fn awaiting_join_lobby_serialize() {
        assert_serialize!(AwaitingJoinLobbyServerMessage::JoinedLobby, vec![1 << 4]);
        assert_serialize!(AwaitingJoinLobbyServerMessage::LobbyFull, vec![1 << 4 | 1]);
        assert_serialize!(
            AwaitingJoinLobbyServerMessage::LobbyNotFound,
            vec![1 << 4 | 2]
        );
    }

    #[test]
    fn awaiting_join_lobby_deserialize_ok() {
        assert_deserialize!(
            AwaitingJoinLobbyServerMessage,
            [1 << 4],
            Ok(AwaitingJoinLobbyServerMessage::JoinedLobby),
        );
        assert_deserialize!(
            AwaitingJoinLobbyServerMessage,
            [1 << 4 | 1],
            Ok(AwaitingJoinLobbyServerMessage::LobbyFull),
        );
        assert_deserialize!(
            AwaitingJoinLobbyServerMessage,
            [1 << 4 | 2],
            Ok(AwaitingJoinLobbyServerMessage::LobbyNotFound),
        );
    }

    #[test]
    fn awaiting_join_lobby_deserialize_err() {
        // empty message.
        assert_deserialize!(
            AwaitingJoinLobbyServerMessage,
            [],
            Err(DeserializeMessageError::EmptyMessage),
        );
        // joined lobby message with extra bytes.
        assert_deserialize!(
            AwaitingJoinLobbyServerMessage,
            [&[1 << 4 | 0], "A5EZ".as_bytes()].concat(),
            Err(DeserializeMessageError::InvalidByteCount),
        );
        // lobby full message with extra bytes.
        assert_deserialize!(
            AwaitingJoinLobbyServerMessage,
            [&[1 << 4 | 1], "A5EZ".as_bytes()].concat(),
            Err(DeserializeMessageError::InvalidByteCount),
        );
        // lobby not found message with extra bytes.
        assert_deserialize!(
            AwaitingJoinLobbyServerMessage,
            [&[1 << 4 | 2], "A5EZ".as_bytes()].concat(),
            Err(DeserializeMessageError::InvalidByteCount),
        );
        // invalid state variant.
        assert_deserialize!(
            AwaitingJoinLobbyServerMessage,
            [0],
            Err(DeserializeMessageError::InvalidState),
        );
        // unrecognised message variant.
        assert_deserialize!(
            AwaitingJoinLobbyServerMessage,
            [1 << 4 | 3],
            Err(DeserializeMessageError::UnrecognisedMessageVariant),
        );
    }

    #[test]
    fn awaiting_opponent_join_serialize() {
        assert_serialize!(
            AwaitingOpponentJoinServerMessage::OpponentJoined,
            vec![2 << 4]
        );
    }

    #[test]
    fn awaiting_opponent_join_deserialize_ok() {
        assert_deserialize!(
            AwaitingOpponentJoinServerMessage,
            [2 << 4],
            Ok(AwaitingOpponentJoinServerMessage::OpponentJoined),
        );
    }

    #[test]
    fn awaiting_opponent_join_deserialize_err() {
        // empty message.
        assert_deserialize!(
            AwaitingOpponentJoinServerMessage,
            [],
            Err(DeserializeMessageError::EmptyMessage),
        );
        // extra bytes.
        assert_deserialize!(
            AwaitingOpponentJoinServerMessage,
            [2 << 4, 0],
            Err(DeserializeMessageError::InvalidByteCount),
        );
        // invalid state variant.
        assert_deserialize!(
            AwaitingOpponentJoinServerMessage,
            [0],
            Err(DeserializeMessageError::InvalidState),
        );
        // unrecognised message variant.
        assert_deserialize!(
            AwaitingOpponentJoinServerMessage,
            [2 << 4 | 1],
            Err(DeserializeMessageError::UnrecognisedMessageVariant),
        );
    }

    #[test]
    fn awaiting_ready_serialize() {
        assert_serialize!(AwaitingReadyServerMessage::OpponentLeft, vec![3 << 4]);
        assert_serialize!(
            AwaitingReadyServerMessage::OpponentReadied,
            vec![3 << 4 | 1]
        );
        assert_serialize!(
            AwaitingReadyServerMessage::OpponentUnreadied,
            vec![3 << 4 | 2]
        );
        assert_serialize!(AwaitingReadyServerMessage::YouReadied, vec![3 << 4 | 3]);
        assert_serialize!(AwaitingReadyServerMessage::YouUnreadied, vec![3 << 4 | 4]);
        assert_serialize!(AwaitingReadyServerMessage::GameStarted, vec![3 << 4 | 5]);
    }

    #[test]
    fn awaiting_ready_deserialize_ok() {
        assert_deserialize!(
            AwaitingReadyServerMessage,
            [3 << 4],
            Ok(AwaitingReadyServerMessage::OpponentLeft),
        );
        assert_deserialize!(
            AwaitingReadyServerMessage,
            [3 << 4 | 1],
            Ok(AwaitingReadyServerMessage::OpponentReadied),
        );
        assert_deserialize!(
            AwaitingReadyServerMessage,
            [3 << 4 | 2],
            Ok(AwaitingReadyServerMessage::OpponentUnreadied),
        );
        assert_deserialize!(
            AwaitingReadyServerMessage,
            [3 << 4 | 3],
            Ok(AwaitingReadyServerMessage::YouReadied),
        );
        assert_deserialize!(
            AwaitingReadyServerMessage,
            [3 << 4 | 4],
            Ok(AwaitingReadyServerMessage::YouUnreadied),
        );
        assert_deserialize!(
            AwaitingReadyServerMessage,
            [3 << 4 | 5],
            Ok(AwaitingReadyServerMessage::GameStarted),
        );
    }

    #[test]
    fn awaiting_ready_deserialize_err() {
        // empty message.
        assert_deserialize!(
            AwaitingReadyServerMessage,
            [],
            Err(DeserializeMessageError::EmptyMessage),
        );
        // extra bytes.
        assert_deserialize!(
            AwaitingReadyServerMessage,
            [3 << 4, 0],
            Err(DeserializeMessageError::InvalidByteCount),
        );
        // extra bytes.
        assert_deserialize!(
            AwaitingReadyServerMessage,
            [3 << 4 | 1, 0],
            Err(DeserializeMessageError::InvalidByteCount),
        );
        // extra bytes.
        assert_deserialize!(
            AwaitingReadyServerMessage,
            [3 << 4 | 2, 0],
            Err(DeserializeMessageError::InvalidByteCount),
        );
        // extra bytes.
        assert_deserialize!(
            AwaitingReadyServerMessage,
            [3 << 4 | 3, 0],
            Err(DeserializeMessageError::InvalidByteCount),
        );
        // extra bytes.
        assert_deserialize!(
            AwaitingReadyServerMessage,
            [3 << 4 | 4, 0],
            Err(DeserializeMessageError::InvalidByteCount),
        );
        // extra bytes.
        assert_deserialize!(
            AwaitingReadyServerMessage,
            [3 << 4 | 5, 0],
            Err(DeserializeMessageError::InvalidByteCount),
        );
        // invalid state variant.
        assert_deserialize!(
            AwaitingReadyServerMessage,
            [0],
            Err(DeserializeMessageError::InvalidState),
        );
        // unrecognised message variant.
        assert_deserialize!(
            AwaitingReadyServerMessage,
            [3 << 4 | 6],
            Err(DeserializeMessageError::UnrecognisedMessageVariant),
        );
    }

    #[test]
    fn playing_serialize() {
        assert_serialize!(PlayingServerMessage::OpponentLeft, vec![4 << 4]);
        assert_serialize!(PlayingServerMessage::OpponentWon, vec![4 << 4 | 1]);
        assert_serialize!(PlayingServerMessage::YouWon, vec![4 << 4 | 2]);
        assert_serialize!(
            PlayingServerMessage::GameStateUpdated {
                game_state: GameState {
                    left_paddle: 0b00000011,  // 3
                    right_paddle: 0b00000111, // 7
                    ball: Ball {
                        x: 0b00001110, // 14
                        y: 0b00000101, // 5
                        moving_right: true,
                        moving_down: false,
                    }
                }
            },
            vec![4 << 4 | 3, 0b00110111, 0b00011101, 0b00001010],
        );
        // these positions are technically impossible given the size of the game window. bits will be truncated during serialized.
        assert_serialize!(
            PlayingServerMessage::GameStateUpdated {
                game_state: GameState {
                    left_paddle: 0b10110111,
                    right_paddle: 0b01110101,
                    ball: Ball {
                        x: 0b11100010,
                        y: 0b10101110,
                        moving_right: false,
                        moving_down: true,
                    }
                }
            },
            vec![4 << 4 | 3, 0b01110101, 0b11000100, 0b01011101],
        );
    }

    #[test]
    fn playing_deserialize_ok() {
        assert_deserialize!(
            PlayingServerMessage,
            [4 << 4],
            Ok(PlayingServerMessage::OpponentLeft),
        );
        assert_deserialize!(
            PlayingServerMessage,
            [4 << 4 | 1],
            Ok(PlayingServerMessage::OpponentWon),
        );
        assert_deserialize!(
            PlayingServerMessage,
            [4 << 4 | 2],
            Ok(PlayingServerMessage::YouWon)
        );
        assert_deserialize!(
            PlayingServerMessage,
            [4 << 4 | 3, 0b01010000, 0b01001111, 0b00010000],
            Ok(PlayingServerMessage::GameStateUpdated {
                game_state: GameState {
                    left_paddle: 0b0101,
                    right_paddle: 0b0000,
                    ball: Ball {
                        x: 0b00100111,
                        y: 0b00001000,
                        moving_right: true,
                        moving_down: false,
                    }
                }
            }),
        );
    }

    #[test]
    fn playing_deserialize_err() {
        // empty message.
        assert_deserialize!(
            PlayingServerMessage,
            [],
            Err(DeserializeMessageError::EmptyMessage),
        );
        // extra bytes.
        assert_deserialize!(
            PlayingServerMessage,
            [4 << 4, 0],
            Err(DeserializeMessageError::InvalidByteCount),
        );
        // extra bytes.
        assert_deserialize!(
            PlayingServerMessage,
            [4 << 4 | 1, 0],
            Err(DeserializeMessageError::InvalidByteCount),
        );
        // extra bytes.
        assert_deserialize!(
            PlayingServerMessage,
            [4 << 4 | 2, 0],
            Err(DeserializeMessageError::InvalidByteCount),
        );
        // extra bytes.
        assert_deserialize!(
            PlayingServerMessage,
            [4 << 4 | 3, 0b11110000, 0b01101111, 0b00011000, 0],
            Err(DeserializeMessageError::InvalidByteCount),
        );
        // invalid left paddle position.
        assert_deserialize!(
            PlayingServerMessage,
            [4 << 4 | 3, 0b11110000, 0b01001111, 0b00010000],
            Err(DeserializeMessageError::InvalidPaddlePosition),
        );
        // invalid right paddle position.
        assert_deserialize!(
            PlayingServerMessage,
            [4 << 4 | 3, 0b01011000, 0b01001111, 0b00010000],
            Err(DeserializeMessageError::InvalidPaddlePosition),
        );
        // invalid ball x position.
        assert_deserialize!(
            PlayingServerMessage,
            [4 << 4 | 3, 0b01010000, 0b01101111, 0b00010000],
            Err(DeserializeMessageError::InvalidBallPosition),
        );
        // invalid ball y position.
        assert_deserialize!(
            PlayingServerMessage,
            [4 << 4 | 3, 0b01010000, 0b01001111, 0b00011000],
            Err(DeserializeMessageError::InvalidBallPosition),
        );
        // invalid state variant.
        assert_deserialize!(
            PlayingServerMessage,
            [0],
            Err(DeserializeMessageError::InvalidState),
        );
        // unrecognised message variant.
        assert_deserialize!(
            PlayingServerMessage,
            [4 << 4 | 4],
            Err(DeserializeMessageError::UnrecognisedMessageVariant)
        );
    }

    #[test]
    fn serialize_and_back() {
        assert_serialize_and_back!(AwaitingNewLobbyServerMessage::NewLobbyCreated {
            lobby_id: "G16P"
        });
        assert_serialize_and_back!(AwaitingJoinLobbyServerMessage::JoinedLobby);
        assert_serialize_and_back!(AwaitingJoinLobbyServerMessage::LobbyFull);
        assert_serialize_and_back!(AwaitingJoinLobbyServerMessage::LobbyNotFound);
        assert_serialize_and_back!(AwaitingOpponentJoinServerMessage::OpponentJoined);
        assert_serialize_and_back!(AwaitingReadyServerMessage::OpponentLeft);
        assert_serialize_and_back!(AwaitingReadyServerMessage::OpponentReadied);
        assert_serialize_and_back!(AwaitingReadyServerMessage::OpponentUnreadied);
        assert_serialize_and_back!(AwaitingReadyServerMessage::YouReadied);
        assert_serialize_and_back!(AwaitingReadyServerMessage::YouUnreadied);
        assert_serialize_and_back!(PlayingServerMessage::OpponentLeft);
        assert_serialize_and_back!(PlayingServerMessage::OpponentWon);
        assert_serialize_and_back!(PlayingServerMessage::YouWon);
        assert_serialize_and_back!(PlayingServerMessage::GameStateUpdated {
            game_state: GameState {
                left_paddle: 6,
                right_paddle: 2,
                ball: Ball {
                    x: 31,
                    y: 10,
                    moving_right: true,
                    moving_down: false,
                },
            },
        });
    }
}
