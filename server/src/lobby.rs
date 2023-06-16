use std::net::TcpStream;

use shared::game_state::GameState;

pub enum Lobby {
    AwaitingJoin {
        host_player_conn: TcpStream,
    },
    Joined {
        left_player_conn: TcpStream,
        right_player_conn: TcpStream,
        state: LobbyState,
    },
}

pub enum LobbyState {
    AwaitingReadies {
        left_player_ready: bool,
        right_player_ready: bool,
    },
    Playing {
        game_state: GameState,
    },
}
