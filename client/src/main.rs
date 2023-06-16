use std::{
    io::stdout,
    sync::mpsc::channel,
    thread::{spawn, Builder},
};

use clap::{Parser, Subcommand};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{self, disable_raw_mode, enable_raw_mode},
};
use tcp_client::TcpClient;

mod tcp_client;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Start,
}

#[derive(Subcommand)]
enum Start {
    /// Start a new game
    New,
    /// Join an existing game
    Join { lobby_id: String },
}

fn main() {
    let cli = Cli::parse();
    enable_raw_mode().unwrap();
    execute!(
        stdout(),
        terminal::EnterAlternateScreen,
        cursor::Hide,
        cursor::MoveTo(0, 0)
    )
    .unwrap();
    let (game_over_tx, game_over_rx) = channel();
    let (ready_key_tx, ready_key_rx) = channel();
    let (move_key_tx, move_key_rx) = channel();
    let game_over_tx_clone = game_over_tx.clone();
    spawn(move || {
        let game_over_tx = game_over_tx_clone.clone();
        if let Err(_) = Builder::new()
            .name("tcp_client".to_owned())
            .spawn(move || {
                TcpClient::run(
                    "127.0.0.1:8080",
                    cli.command,
                    game_over_tx,
                    ready_key_rx,
                    move_key_rx,
                )
            })
            .unwrap()
            .join()
        {
            let _ = game_over_tx_clone.send(Quit::Panic);
        }
    });
    Builder::new()
        .name("terminate_key_listener".to_owned())
        .spawn(move || loop {
            let event = event::read().unwrap();
            if let Event::Key(key_event) = event {
                if key_event.modifiers == KeyModifiers::CONTROL
                    && key_event.code == KeyCode::Char('c')
                {
                    let _ = game_over_tx.send(Quit::CtrlC);
                } else if key_event.modifiers == KeyModifiers::NONE {
                    match key_event.code {
                        KeyCode::Char('r') => {
                            let _ = ready_key_tx.send(());
                        }
                        KeyCode::Down => {
                            let _ = move_key_tx.send(true);
                        }
                        KeyCode::Up => {
                            let _ = move_key_tx.send(false);
                        }
                        _ => {}
                    }
                }
            }
        })
        .unwrap();
    let game_over = game_over_rx.recv().unwrap();
    disable_raw_mode().unwrap();
    execute!(stdout(), terminal::LeaveAlternateScreen, cursor::Show).unwrap();
    match game_over {
        Quit::CtrlC => println!("^C"),
        Quit::Panic => println!("error occurred"),
        Quit::LobbyFull => println!("lobby full"),
        Quit::LobbyNotFound => println!("lobby not found"),
        Quit::YouWon => println!("you won"),
        Quit::OpponentWon => println!("you lost"),
        Quit::OpponentLeft => println!("opponent left"),
    }
}

enum Quit {
    CtrlC,
    Panic,
    LobbyFull,
    LobbyNotFound,
    YouWon,
    OpponentWon,
    OpponentLeft,
}
