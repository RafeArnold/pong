use std::{
    net::TcpListener,
    sync::{Arc, Mutex},
    thread::Builder,
};

use dashmap::DashMap;
use rand::RngCore;
use shared::LobbyId;

use crate::{
    lobby::Lobby, lobby_id_generator::LobbyIdGenerator, tcp_stream_handler::TcpStreamHandler,
};

struct TcpServer {
    inner: TcpListener,
    lobbies: Arc<DashMap<LobbyId, Lobby>>,
    lobby_id_generator: Arc<Mutex<LobbyIdGenerator>>,
}

pub fn start() {
    let server = TcpListener::bind("127.0.0.1:8080").expect("failed to start server");
    println!("server started");
    TcpServer::new(server).handle_incoming();
}

impl TcpServer {
    pub fn new(inner: TcpListener) -> Self {
        let lobbies = Arc::new(DashMap::new());
        // no data within the application is persisted or distributed outside the application, so
        // randomly generating a new key on each startup is acceptable.
        let mut key = [0; 32];
        rand::thread_rng().fill_bytes(&mut key);
        let lobby_id_generator = Arc::new(Mutex::new(LobbyIdGenerator::new(&key)));
        Self {
            inner,
            lobbies,
            lobby_id_generator,
        }
    }

    fn handle_incoming(&self) {
        println!("listening for incoming connections!");
        for stream in self.inner.incoming() {
            match stream {
                Ok(stream) => {
                    let peer_addr = match stream.peer_addr() {
                        Ok(peer_addr) => peer_addr,
                        Err(err) => {
                            eprintln!("failed to retrieve peer address of connection: {err}");
                            continue;
                        }
                    };
                    println!("connection established from {:?}", peer_addr);
                    let lobbies = self.lobbies.clone();
                    let lobby_id_generator = self.lobby_id_generator.clone();
                    Builder::new()
                        .name(format!("handler_{peer_addr}"))
                        .spawn(move || {
                            TcpStreamHandler::new(stream, lobbies, lobby_id_generator)
                                .handle_stream()
                        })
                        .unwrap();
                }
                Err(err) => eprintln!("incoming connection failure: {err}"),
            }
        }
    }
}
