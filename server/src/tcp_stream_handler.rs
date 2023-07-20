use std::{
    io::{Read, Write},
    net::TcpStream,
    sync::{Arc, Mutex},
    thread::{sleep, Builder},
    time::Duration,
};

use dashmap::{mapref::entry::Entry, DashMap};

use shared::{
    client_msg::{
        AwaitingOpenClientMessage, AwaitingReadyClientMessage, PlayingClientMessage,
        MAX_CLIENT_MESSAGE_SIZE,
    },
    game_state::{Ball, GameState, GAME_HEIGHT, GAME_WIDTH, PADDLE_HEIGHT},
    server_msg::{
        AwaitingJoinLobbyServerMessage, AwaitingNewLobbyServerMessage,
        AwaitingOpponentJoinServerMessage, AwaitingReadyServerMessage, PlayingServerMessage,
        SERVER_MESSAGE_DELIMITER,
    },
    LobbyId,
};

use crate::{
    lobby::{Lobby, LobbyState},
    lobby_id_generator::LobbyIdGenerator,
};

pub struct TcpStreamHandler {
    stream: TcpStream,
    lobbies: Arc<DashMap<LobbyId, Lobby>>,
    lobby_id_generator: Arc<Mutex<LobbyIdGenerator>>,
    lobby_id: Option<String>,
}

impl TcpStreamHandler {
    pub fn new(
        stream: TcpStream,
        lobbies: Arc<DashMap<LobbyId, Lobby>>,
        lobby_id_generator: Arc<Mutex<LobbyIdGenerator>>,
    ) -> Self {
        Self {
            stream,
            lobbies,
            lobby_id_generator,
            lobby_id: None,
        }
    }

    pub fn handle_stream(&mut self) {
        let mut buffer = [0; MAX_CLIENT_MESSAGE_SIZE];
        loop {
            match self.stream.read(&mut buffer) {
                Ok(n) => {
                    if n == 0 {
                        println!("connection {:?} closed", self.stream.peer_addr().unwrap());
                        if let Some(lobby_id) = &self.lobby_id {
                            let lobby = self.lobbies.remove(lobby_id);
                            if let Some((_, lobby)) = lobby {
                                match lobby {
                                    Lobby::AwaitingJoin { .. } => {}
                                    Lobby::Joined {
                                        left_player_conn,
                                        right_player_conn,
                                        state,
                                    } => {
                                        let is_left_player = self.stream.peer_addr().unwrap()
                                            == left_player_conn.peer_addr().unwrap();
                                        let mut opponent_conn = if is_left_player {
                                            right_player_conn
                                        } else {
                                            left_player_conn
                                        };
                                        match state {
                                            LobbyState::AwaitingReadies { .. } => {
                                                Self::write_to_client(
                                                    AwaitingReadyServerMessage::OpponentLeft,
                                                    &mut opponent_conn,
                                                );
                                            }
                                            LobbyState::Playing { .. } => {
                                                Self::write_to_client(
                                                    PlayingServerMessage::OpponentLeft,
                                                    &mut opponent_conn,
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        break;
                    }
                    println!(
                        "received msg from client {}: {:?}",
                        self.stream.peer_addr().unwrap(),
                        &buffer[..n]
                    );
                    self.handle_client_message(&buffer[..n]);
                }
                Err(err) => eprintln!(
                    "failed to read from {:?}: {err}",
                    self.stream.peer_addr().unwrap(),
                ),
            };
        }
    }

    fn handle_client_message(&mut self, message: &[u8]) {
        match self
            .lobby_id
            .as_ref()
            .and_then(|lobby_id| self.lobbies.get_mut(lobby_id))
        {
            Some(mut lobby) => {
                match lobby.value_mut() {
                    Lobby::AwaitingJoin { .. } => {
                        eprintln!("received message from client during invalid state")
                    }
                    Lobby::Joined {
                        left_player_conn,
                        right_player_conn,
                        state,
                    } => {
                        let is_left_player = self.stream.peer_addr().unwrap()
                            == left_player_conn.peer_addr().unwrap();
                        match state {
                            LobbyState::AwaitingReadies {
                                left_player_ready,
                                right_player_ready,
                            } => {
                                let message = match AwaitingReadyClientMessage::try_from(message) {
                                    Ok(message) => message,
                                    Err(err) => {
                                        eprintln!("failed to deserialise client message: {err}");
                                        return;
                                    }
                                };
                                let is_ready = match message {
                                    AwaitingReadyClientMessage::Ready => true,
                                    AwaitingReadyClientMessage::Unready => false,
                                };
                                if is_left_player {
                                    *left_player_ready = is_ready;
                                } else {
                                    *right_player_ready = is_ready;
                                }
                                Self::write_to_client(
                                    if is_ready {
                                        AwaitingReadyServerMessage::YouReadied
                                    } else {
                                        AwaitingReadyServerMessage::YouUnreadied
                                    },
                                    &mut self.stream,
                                );
                                if !(*left_player_ready && *right_player_ready) {
                                    let opponent_conn = if is_left_player {
                                        right_player_conn
                                    } else {
                                        left_player_conn
                                    };
                                    Self::write_to_client(
                                        if is_ready {
                                            AwaitingReadyServerMessage::OpponentReadied
                                        } else {
                                            AwaitingReadyServerMessage::OpponentUnreadied
                                        },
                                        opponent_conn,
                                    );
                                } else {
                                    // both players are ready. start the game.
                                    let paddle_starting_position = 0;
                                    // GAME_HEIGHT / 2 - PADDLE_HEIGHT / 2;
                                    let game_state = GameState {
                                        left_paddle: paddle_starting_position,
                                        right_paddle: paddle_starting_position,
                                        ball: Ball {
                                            x: GAME_WIDTH / 2,
                                            y: GAME_HEIGHT / 2,
                                            moving_right: true,
                                            moving_down: true,
                                        },
                                    };
                                    *state = LobbyState::Playing {
                                        game_state: game_state.clone(),
                                    };
                                    Self::write_to_client(
                                        AwaitingReadyServerMessage::GameStarted,
                                        &mut self.stream,
                                    );
                                    let opponent_conn = if is_left_player {
                                        right_player_conn
                                    } else {
                                        left_player_conn
                                    };
                                    Self::write_to_client(
                                        AwaitingReadyServerMessage::GameStarted,
                                        opponent_conn,
                                    );
                                    let game_state_msg =
                                        PlayingServerMessage::GameStateUpdated { game_state };
                                    Self::write_to_client(game_state_msg.clone(), &mut self.stream);
                                    Self::write_to_client(game_state_msg, opponent_conn);
                                    let lobby_id = self.lobby_id.clone().unwrap();
                                    let lobbies_clone = Arc::clone(&self.lobbies);
                                    Builder::new()
                                        .name(format!("ball_handler_{lobby_id}"))
                                        .spawn(move || {
                                            loop {
                                                sleep(Duration::from_millis(100));
                                                match lobbies_clone.get_mut(&lobby_id) {
                                                    Some(mut entry) => match entry.value_mut() {
                                                        Lobby::AwaitingJoin { .. } | Lobby::Joined { state: LobbyState::AwaitingReadies { .. }, .. } => {
                                                            eprintln!("lobby is in the incorrect state to update game state");
                                                            return;
                                                        },
                                                        Lobby::Joined { left_player_conn, right_player_conn, state: LobbyState::Playing { game_state } } => {
                                                            let left_paddle = game_state.left_paddle;
                                                            let right_paddle = game_state.right_paddle;
                                                            let ball = &mut game_state.ball;
                                                            if ball.x == 1 {
                                                                if left_paddle > ball.y || left_paddle + PADDLE_HEIGHT <= ball.y {
                                                                    Self::write_to_client(PlayingServerMessage::OpponentWon, left_player_conn);
                                                                    Self::write_to_client(PlayingServerMessage::YouWon, right_player_conn);
                                                                } else {
                                                                    ball.moving_right = !ball.moving_right;
                                                                }
                                                            }
                                                            if ball.x == GAME_WIDTH - 2 {
                                                                if right_paddle > ball.y || right_paddle + PADDLE_HEIGHT <= ball.y {
                                                                    Self::write_to_client(PlayingServerMessage::YouWon, left_player_conn);
                                                                    Self::write_to_client(PlayingServerMessage::OpponentWon, right_player_conn);
                                                                } else {
                                                                    ball.moving_right = !ball.moving_right;
                                                                }
                                                            }
                                                            if ball.y == 0 || ball.y == GAME_HEIGHT - 1 {
                                                                ball.moving_down = !ball.moving_down;
                                                            }
                                                            if ball.moving_right {
                                                                ball.x += 1;
                                                            } else {
                                                                ball.x -= 1;
                                                            }
                                                            if ball.moving_down {
                                                                ball.y += 1;
                                                            } else {
                                                                ball.y -= 1;
                                                            }
                                                            let msg = PlayingServerMessage::GameStateUpdated { game_state: game_state.clone() };
                                                            Self::write_to_client(msg.clone(), left_player_conn);
                                                            Self::write_to_client(msg, right_player_conn);
                                                        },
                                                    },
                                                    None => {
                                                        println!("closing ball handler for lobby {lobby_id}");
                                                        return;
                                                    },
                                                }
                                            }
                                        })
                                        .unwrap();
                                }
                            }
                            LobbyState::Playing { game_state } => {
                                let message = match PlayingClientMessage::try_from(message) {
                                    Ok(message) => message,
                                    Err(err) => {
                                        eprintln!("failed to deserialise client message: {err}");
                                        return;
                                    }
                                };
                                match message {
                                    PlayingClientMessage::MovePaddle { pos } => {
                                        if is_left_player {
                                            game_state.left_paddle = pos;
                                        } else {
                                            game_state.right_paddle = pos;
                                        }
                                    }
                                }
                                let reply = PlayingServerMessage::GameStateUpdated {
                                    game_state: game_state.clone(),
                                };
                                Self::write_to_client(reply.clone(), &mut self.stream);
                                let opponent_conn = if is_left_player {
                                    right_player_conn
                                } else {
                                    left_player_conn
                                };
                                Self::write_to_client(reply, opponent_conn);
                            }
                        }
                    }
                }
            }
            None => {
                match AwaitingOpenClientMessage::try_from(message) {
                    Ok(AwaitingOpenClientMessage::NewLobby) => {
                        // create a new lobby.
                        let lobby_id = self.lobby_id_generator.lock().unwrap().next_id();
                        let mut stream = self.stream.try_clone().unwrap();
                        let lobby = Lobby::AwaitingJoin {
                            host_player_conn: stream.try_clone().unwrap(),
                        };
                        // TODO: handle if a lobby already exists with this id (probably close any connections to the old lobby, or keep generating ids until one works).
                        self.lobbies.insert(lobby_id.to_owned(), lobby);
                        self.lobby_id = Some(lobby_id.to_owned());
                        let reply = AwaitingNewLobbyServerMessage::NewLobbyCreated {
                            lobby_id: &lobby_id,
                        };
                        Self::write_to_client(reply, &mut stream);
                    }
                    Ok(AwaitingOpenClientMessage::JoinLobby { lobby_id }) => {
                        match self.lobbies.entry(lobby_id.to_owned()) {
                            Entry::Occupied(entry) => match entry.get() {
                                Lobby::AwaitingJoin { host_player_conn } => {
                                    let host_player_conn = host_player_conn.try_clone().unwrap();
                                    let mut stream = self.stream.try_clone().unwrap();
                                    let lobby = Lobby::Joined {
                                        left_player_conn: host_player_conn.try_clone().unwrap(),
                                        right_player_conn: stream.try_clone().unwrap(),
                                        state: LobbyState::AwaitingReadies {
                                            left_player_ready: false,
                                            right_player_ready: false,
                                        },
                                    };
                                    self.lobby_id = Some(lobby_id.to_owned());
                                    entry.replace_entry(lobby);
                                    Self::write_to_client(
                                        AwaitingJoinLobbyServerMessage::JoinedLobby,
                                        &mut stream,
                                    );
                                    let mut opponent_conn = host_player_conn;
                                    Self::write_to_client(
                                        AwaitingOpponentJoinServerMessage::OpponentJoined,
                                        &mut opponent_conn,
                                    );
                                }
                                Lobby::Joined { .. } => {
                                    Self::write_to_client(
                                        AwaitingJoinLobbyServerMessage::LobbyFull,
                                        &mut self.stream,
                                    );
                                    // TODO: shutdown connection
                                }
                            },
                            Entry::Vacant(_) => {
                                Self::write_to_client(
                                    AwaitingJoinLobbyServerMessage::LobbyNotFound,
                                    &mut self.stream,
                                );
                                // TODO: shutdown connection
                            }
                        }
                    }
                    Err(err) => eprintln!("failed to deserialize client message: {err}"),
                }
            }
        };
    }

    fn write_to_client<T: Into<Vec<u8>>>(message: T, stream: &mut TcpStream) {
        let mut message: Vec<u8> = message.into();
        message.push(SERVER_MESSAGE_DELIMITER);
        if let Some(err) = stream.write_all(message.as_slice()).err() {
            eprintln!(
                "failed to write message {:?} to client {}: {err}",
                message,
                stream.peer_addr().unwrap()
            );
        }
    }
}
