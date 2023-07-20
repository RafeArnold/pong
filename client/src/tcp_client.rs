use std::{
    error::Error,
    fmt::Display,
    io::{stdout, BufRead, BufReader, Stdout, StdoutLock, Write},
    net::TcpStream,
    sync::{
        atomic::{AtomicU8, Ordering},
        mpsc::{channel, Receiver, Sender, TryRecvError},
        Arc,
    },
    thread::Builder,
};

use crossterm::{
    cursor::{MoveDown, MoveLeft, MoveRight, MoveToColumn, MoveToNextLine, MoveUp},
    execute,
    style::{Color, Print, SetForegroundColor},
    terminal::{Clear, ClearType},
};
use shared::{
    client_msg::{AwaitingOpenClientMessage, AwaitingReadyClientMessage, PlayingClientMessage},
    game_state::{Ball, GAME_HEIGHT, GAME_WIDTH, PADDLE_HEIGHT},
    server_msg::{
        AwaitingJoinLobbyServerMessage, AwaitingNewLobbyServerMessage,
        AwaitingOpponentJoinServerMessage, AwaitingReadyServerMessage, PlayingServerMessage,
        MAX_SERVER_MESSAGE_SIZE, SERVER_MESSAGE_DELIMITER,
    },
    DeserializeMessageError,
};

use crate::{Quit, Start};

pub struct TcpClient {
    stream: BufReader<TcpStream>,
    server_msg_buffer: Vec<u8>,
    is_left_player: bool,
    game_over_tx: Sender<Quit>,
}

impl TcpClient {
    fn new(stream: TcpStream, is_left_player: bool, game_over_tx: Sender<Quit>) -> Self {
        Self {
            stream: BufReader::with_capacity(MAX_SERVER_MESSAGE_SIZE, stream.try_clone().unwrap()),
            server_msg_buffer: Vec::with_capacity(MAX_SERVER_MESSAGE_SIZE),
            is_left_player,
            game_over_tx,
        }
    }

    pub(crate) fn run(
        server_addr: &str,
        start: Start,
        game_over_tx: Sender<Quit>,
        ready_key_rx: Receiver<()>,
        move_key_rx: Receiver<bool>,
    ) {
        let stream = TcpStream::connect(server_addr).expect("failed to connect to server");
        let is_left_player = match start {
            Start::New => true,
            Start::Join { .. } => false,
        };
        let mut client = Self::new(stream, is_left_player, game_over_tx.clone());
        let mut stdout = stdout();
        draw_barriers(&mut stdout);
        execute!(stdout, MoveDown(2)).unwrap();
        match start {
            Start::New => {
                let message = AwaitingOpenClientMessage::NewLobby;
                Self::send(client.stream.get_mut(), message);
                let lobby_id = match client.await_msg::<AwaitingNewLobbyServerMessage>().unwrap() {
                    AwaitingNewLobbyServerMessage::NewLobbyCreated { lobby_id } => lobby_id,
                };
                let text = format!("lobby id: {lobby_id}");
                execute!(
                    stdout,
                    MoveRight((GAME_WIDTH as u16 - text.len() as u16) / 2),
                    Print(text),
                    MoveToColumn(0),
                )
                .unwrap();
                stdout.flush().unwrap();
                match client
                    .await_msg::<AwaitingOpponentJoinServerMessage>()
                    .unwrap()
                {
                    AwaitingOpponentJoinServerMessage::OpponentJoined => {}
                };
            }
            Start::Join { lobby_id } => {
                let message = AwaitingOpenClientMessage::JoinLobby {
                    lobby_id: &lobby_id,
                };
                Self::send(client.stream.get_mut(), message);
                match client.await_msg().unwrap() {
                    AwaitingJoinLobbyServerMessage::JoinedLobby => {}
                    AwaitingJoinLobbyServerMessage::LobbyFull => {
                        game_over_tx.send(Quit::LobbyFull).unwrap()
                    }
                    AwaitingJoinLobbyServerMessage::LobbyNotFound => {
                        game_over_tx.send(Quit::LobbyNotFound).unwrap()
                    }
                };
            }
        };
        let client = client.await_game_start(game_over_tx.clone(), ready_key_rx);
        if client.is_none() {
            return;
        }
        let mut client = client.unwrap();
        let local_paddle_pos = Arc::new(AtomicU8::new(0));
        let local_paddle_pos_clone = Arc::clone(&local_paddle_pos);
        execute!(stdout, MoveUp(2)).unwrap();
        draw_game(
            stdout.lock(),
            0,
            0,
            Ball {
                x: GAME_WIDTH / 2,
                y: GAME_HEIGHT / 2,
                moving_right: true,
                moving_down: true,
            },
        );
        let mut stream_writer_clone = client.stream.get_ref().try_clone().unwrap();
        // drain previously buffered move key events.
        while let Ok(_) = move_key_rx.try_recv() {}
        Builder::new()
            .name("move_key_listener".to_owned())
            .spawn(move || {
                for move_key in move_key_rx {
                    let new_pos = local_paddle_pos_clone.fetch_update(
                        Ordering::Relaxed,
                        Ordering::Relaxed,
                        |pos| {
                            if move_key && pos < GAME_HEIGHT - PADDLE_HEIGHT {
                                // move down.
                                Some(pos + 1)
                            } else if !move_key && pos > 0 {
                                // move up.
                                Some(pos - 1)
                            } else {
                                None
                            }
                        },
                    );
                    if let Ok(prev_pos) = new_pos {
                        let new_pos = if move_key {
                            // move down.
                            prev_pos + 1
                        } else {
                            // move up.
                            prev_pos - 1
                        };
                        Self::send(
                            &mut stream_writer_clone,
                            PlayingClientMessage::MovePaddle { pos: new_pos },
                        )
                    }
                }
            })
            .unwrap();
        loop {
            let message = client.await_msg::<PlayingServerMessage>().unwrap();
            match message {
                PlayingServerMessage::OpponentLeft => {
                    game_over_tx.send(Quit::OpponentLeft).unwrap()
                }
                PlayingServerMessage::OpponentWon => {
                    let _ = client.game_over_tx.send(Quit::OpponentWon);
                    break;
                }
                PlayingServerMessage::YouWon => {
                    let _ = client.game_over_tx.send(Quit::YouWon);
                    break;
                }
                PlayingServerMessage::GameStateUpdated { game_state } => {
                    local_paddle_pos.store(
                        if client.is_left_player {
                            game_state.left_paddle
                        } else {
                            game_state.right_paddle
                        },
                        Ordering::Relaxed,
                    );
                    let mut stdout = stdout.lock();
                    execute!(stdout, MoveUp(GAME_HEIGHT as u16)).unwrap();
                    draw_game(
                        stdout,
                        game_state.left_paddle,
                        game_state.right_paddle,
                        game_state.ball,
                    );
                }
            }
        }
    }

    fn await_game_start(
        mut self,
        game_over_tx: Sender<Quit>,
        ready_key_rx: Receiver<()>,
    ) -> Option<Self> {
        let is_left_player = self.is_left_player;
        let mut stdout = stdout();
        execute!(
            stdout,
            Clear(ClearType::CurrentLine),
            MoveRight((GAME_WIDTH as u16 - 32) / 2),
            Print("press 'r' to toggle ready status"),
            MoveToNextLine(1),
        )
        .unwrap();
        execute!(stdout, SetForegroundColor(Color::Red)).unwrap();
        if is_left_player {
            execute!(
                stdout,
                Print("you are not ready"),
                MoveRight(GAME_WIDTH as u16 - (21 + 17)),
                Print("opponent is not ready"),
            )
            .unwrap();
        } else {
            execute!(
                stdout,
                Print("opponent is not ready"),
                MoveRight(GAME_WIDTH as u16 - (21 + 17)),
                Print("you are not ready"),
            )
            .unwrap();
        }
        execute!(stdout, SetForegroundColor(Color::Reset), MoveToColumn(0)).unwrap();
        stdout.flush().unwrap();
        let (kill_keys_tx, kill_keys_rx) = channel::<()>();
        let (event_tx, event_rx) = channel();
        let event_tx_clone = event_tx.clone();
        // drain previously buffered ready key events.
        while let Ok(_) = ready_key_rx.try_recv() {}
        Builder::new()
            .name("ready_key_listener".to_owned())
            .spawn(move || {
                for _ in ready_key_rx {
                    if let Ok(_) | Err(TryRecvError::Disconnected) = kill_keys_rx.try_recv() {
                        break;
                    }
                    let _ = event_tx_clone.send(AwaitingReadyEvent::ReadyKeyPressed);
                }
            })
            .unwrap();
        let mut stream_writer_clone = self.stream.get_ref().try_clone().unwrap();
        let msg_listener = Builder::new()
            .name("awaiting_ready_msg_listener".to_owned())
            .spawn(move || {
                loop {
                    let msg = self.await_msg::<AwaitingReadyServerMessage>();
                    match msg {
                        Ok(AwaitingReadyServerMessage::GameStarted)
                        | Ok(AwaitingReadyServerMessage::OpponentLeft)
                        | Err(_) => {
                            let _ = event_tx.send(AwaitingReadyEvent::ServerMessageReceived(msg));
                            break;
                        }
                        Ok(AwaitingReadyServerMessage::OpponentReadied)
                        | Ok(AwaitingReadyServerMessage::OpponentUnreadied)
                        | Ok(AwaitingReadyServerMessage::YouReadied)
                        | Ok(AwaitingReadyServerMessage::YouUnreadied) => {
                            let _ = event_tx.send(AwaitingReadyEvent::ServerMessageReceived(msg));
                        }
                    };
                }
                self
            })
            .unwrap();
        let mut you_ready = false;
        let mut awaiting_you_readied_reply = false;
        for event in event_rx.iter() {
            match event {
                AwaitingReadyEvent::ReadyKeyPressed => {
                    if awaiting_you_readied_reply {
                        continue;
                    }
                    awaiting_you_readied_reply = true;
                    Self::send(
                        &mut stream_writer_clone,
                        if you_ready {
                            AwaitingReadyClientMessage::Unready
                        } else {
                            AwaitingReadyClientMessage::Ready
                        },
                    );
                }
                AwaitingReadyEvent::ServerMessageReceived(msg) => {
                    match msg.unwrap() {
                        AwaitingReadyServerMessage::OpponentReadied => {
                            let colour = Color::Green;
                            if is_left_player {
                                display_status_right(&mut stdout, "    opponent is ready", colour);
                            } else {
                                display_status_left(&mut stdout, "opponent is ready    ", colour);
                            }
                        }
                        AwaitingReadyServerMessage::OpponentUnreadied => {
                            let text = "opponent is not ready";
                            let colour = Color::Red;
                            if is_left_player {
                                display_status_right(&mut stdout, text, colour);
                            } else {
                                display_status_left(&mut stdout, text, colour);
                            }
                        }
                        AwaitingReadyServerMessage::YouReadied => {
                            you_ready = true;
                            awaiting_you_readied_reply = false;
                            let colour = Color::Green;
                            if is_left_player {
                                display_status_left(&mut stdout, "you are ready    ", colour);
                            } else {
                                display_status_right(&mut stdout, "    you are ready", colour);
                            }
                        }
                        AwaitingReadyServerMessage::YouUnreadied => {
                            you_ready = false;
                            awaiting_you_readied_reply = false;
                            let text = "you are not ready";
                            let colour = Color::Red;
                            if is_left_player {
                                display_status_left(&mut stdout, text, colour);
                            } else {
                                display_status_right(&mut stdout, text, colour);
                            }
                        }
                        AwaitingReadyServerMessage::GameStarted => {
                            let _ = kill_keys_tx.send(());
                            break;
                        }
                        AwaitingReadyServerMessage::OpponentLeft => {
                            game_over_tx.send(Quit::OpponentLeft).unwrap();
                        }
                    };
                }
            }
        }
        Some(msg_listener.join().unwrap())
    }

    fn send<M>(stream: &mut TcpStream, message: M)
    where
        Vec<u8>: From<M>,
    {
        stream.write_all(&Vec::<u8>::from(message)).unwrap();
    }

    fn await_msg<'a, R>(&'a mut self) -> Result<R, AwaitMsgError>
    where
        R: TryFrom<&'a [u8], Error = DeserializeMessageError>,
    {
        let buffer = &mut self.server_msg_buffer;
        buffer.clear();
        let n = self
            .stream
            .read_until(SERVER_MESSAGE_DELIMITER, buffer)
            .map_err(|err| AwaitMsgError::IOError(err))?;
        if n == 0 {
            return Err(AwaitMsgError::ServerClosedConnection);
        }
        R::try_from(&buffer[..n - 1]).map_err(|err| AwaitMsgError::DeserializeMsg(err))
    }
}

fn draw_game(mut w: StdoutLock, left_paddle: u8, right_paddle: u8, ball: Ball) {
    clear(&mut w);
    execute!(
        w,
        MoveRight(ball.x as u16 + 1),
        MoveLeft(1),
        MoveDown(ball.y as u16 + 1),
        MoveUp(1),
        Print('o'),
        MoveToColumn(0),
        MoveUp(ball.y as u16 + 1),
        MoveDown(1),
    )
    .unwrap();
    draw_paddle(&mut w, left_paddle);
    execute!(
        w,
        MoveUp(GAME_HEIGHT as u16),
        MoveRight(GAME_WIDTH as u16 - 1),
    )
    .unwrap();
    draw_paddle(&mut w, right_paddle);
    execute!(w, MoveToColumn(0)).unwrap();
    w.flush().unwrap();
}

fn clear<W: Write>(w: &mut W) {
    for _ in 0..GAME_HEIGHT {
        execute!(w, Clear(ClearType::CurrentLine), MoveToNextLine(1)).unwrap();
    }
    execute!(w, MoveUp(GAME_HEIGHT as u16)).unwrap();
}

fn draw_barriers<W: Write>(w: &mut W) {
    draw_barrier(w);
    execute!(w, MoveDown(GAME_HEIGHT as u16 + 1)).unwrap();
    draw_barrier(w);
    execute!(w, MoveUp(GAME_HEIGHT as u16 + 1)).unwrap();
}

fn draw_barrier<W: Write>(w: &mut W) {
    for _ in 0..GAME_WIDTH {
        execute!(w, Print("-")).unwrap();
    }
    execute!(w, MoveLeft(GAME_WIDTH as u16)).unwrap();
}

fn draw_paddle<W: Write>(w: &mut W, paddle: u8) {
    for _ in 0..paddle {
        execute!(w, MoveDown(1)).unwrap();
    }
    for _ in 0..PADDLE_HEIGHT {
        execute!(w, Print('|'), MoveLeft(1), MoveDown(1)).unwrap();
    }
    for _ in 0..GAME_HEIGHT - PADDLE_HEIGHT - paddle {
        execute!(w, MoveDown(1)).unwrap();
    }
}

fn display_status_left(stdout: &mut Stdout, text: &str, colour: Color) {
    execute!(
        stdout,
        SetForegroundColor(colour),
        Print(text),
        SetForegroundColor(Color::Reset),
        MoveToColumn(0),
    )
    .unwrap();
    stdout.flush().unwrap();
}

fn display_status_right(stdout: &mut Stdout, text: &str, colour: Color) {
    execute!(
        stdout,
        MoveRight(GAME_WIDTH as u16 - text.len() as u16),
        SetForegroundColor(colour),
        Print(text),
        SetForegroundColor(Color::Reset),
        MoveToColumn(0),
    )
    .unwrap();
    stdout.flush().unwrap();
}

enum AwaitingReadyEvent {
    ReadyKeyPressed,
    ServerMessageReceived(Result<AwaitingReadyServerMessage, AwaitMsgError>),
}

#[derive(Debug)]
enum AwaitMsgError {
    ServerClosedConnection,
    DeserializeMsg(DeserializeMessageError),
    IOError(std::io::Error),
}

impl Display for AwaitMsgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AwaitMsgError::ServerClosedConnection => Display::fmt("server closed connection", f),
            AwaitMsgError::DeserializeMsg(err) => Display::fmt(err, f),
            AwaitMsgError::IOError(err) => Display::fmt(err, f),
        }
    }
}

impl Error for AwaitMsgError {
    fn cause(&self) -> Option<&dyn Error> {
        match self {
            AwaitMsgError::ServerClosedConnection => None,
            AwaitMsgError::DeserializeMsg(err) => Some(err),
            AwaitMsgError::IOError(err) => Some(err),
        }
    }
}
