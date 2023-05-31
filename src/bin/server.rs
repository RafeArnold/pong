use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex,
    },
    thread::{spawn, JoinHandle},
};

fn main() {
    let server = TcpListener::bind("127.0.0.1:8080").expect("failed to start server");
    println!("server started");
    handle_incoming(server)
        .join()
        .expect("failed to handle incoming connections");
}

fn handle_incoming(server: TcpListener) -> JoinHandle<()> {
    let (read_tx, read_rx) = channel();
    let write_channels = Arc::new(Mutex::new(HashMap::new()));
    let write_channels_clone = Arc::clone(&write_channels);
    spawn(move || loop {
        println!("listening for incoming connections!");
        for stream in server.incoming() {
            match stream {
                Ok(stream) => {
                    let connection_id = match stream.peer_addr() {
                        Ok(peer_addr) => peer_addr,
                        Err(err) => {
                            eprintln!("failed to retrieve peer address of connection: {err}");
                            continue;
                        }
                    };
                    let (write_tx, write_rx) = channel();
                    handle_stream(stream, connection_id, read_tx.clone(), write_rx);
                    write_channels_clone
                        .lock()
                        .expect("failed to retrieve channels lock")
                        .insert(connection_id, write_tx);
                }
                Err(err) => eprintln!("incoming connection failure: {err}"),
            }
        }
    });
    spawn(move || loop {
        for message in &read_rx {
            match message {
                ReadMessage::Broadcast {
                    connection_id,
                    data,
                } => {
                    for (_, write_tx) in write_channels
                        .lock()
                        .expect("failed to retrieve channels lock")
                        .iter()
                        .filter(|(other_connection_id, _)| **other_connection_id != connection_id)
                    {
                        match write_tx.send(WriteMessage::Broadcast { data: data.clone() }) {
                            Ok(_) => {
                                println!("successfully sent broadcast to {connection_id}: {data}")
                            }
                            Err(err) => {
                                eprintln!(
                                    "failed to send broadcast {:?} to {connection_id}: {err}",
                                    data
                                );
                            }
                        }
                    }
                }
                ReadMessage::ConnectionClosed { connection_id } => {
                    let mut write_channels = write_channels
                        .lock()
                        .expect("failed to retrieve channels lock");
                    match write_channels.get(&connection_id) {
                        Some(write_tx) => match write_tx.send(WriteMessage::ConnectionClosed) {
                            Ok(_) => {
                                println!("successfully sent connection closed message to {connection_id}");
                            }
                            Err(err) => {
                                eprintln!("failed to send connection closed message to {connection_id}: {err}");
                            }
                        },
                        None => eprintln!("missing write channel for {connection_id}"),
                    }
                    write_channels.remove(&connection_id);
                }
            }
        }
    })
}

fn handle_stream(
    mut stream: TcpStream,
    connection_id: ConnectionId,
    read_tx: Sender<ReadMessage>,
    write_rx: Receiver<WriteMessage>,
) {
    let peer_addr = stream.peer_addr();
    println!("connection established from {:?}", peer_addr);
    let stream_clone = stream
        .try_clone()
        .expect(&format!("failed to clone connection {:?}", peer_addr));
    spawn(move || {
        let stream = stream_clone;
        let peer_addr = stream.peer_addr();
        let mut reader = BufReader::new(stream);
        let mut buffer = String::new();
        loop {
            match reader.read_line(&mut buffer) {
                Ok(n) => {
                    if n == 0 {
                        println!("connection {:?} closed", peer_addr);
                        match read_tx.send(ReadMessage::ConnectionClosed { connection_id }) {
                            Ok(_) => {}
                            Err(err) => {
                                eprintln!("failed to send connection closed message to read receiver: {err}");
                            }
                        }
                        break;
                    }
                    println!("successfully read data from {:?}: {buffer}", peer_addr);
                    match read_tx.send(ReadMessage::Broadcast {
                        connection_id,
                        data: buffer.clone(),
                    }) {
                        Ok(_) => {}
                        Err(err) => {
                            eprintln!("failed to send broadcast {buffer} to read receiver: {err}");
                        }
                    }
                }
                Err(err) => eprintln!("failed to read from {:?}: {err}", peer_addr),
            };
            buffer.clear();
        }
    });
    spawn(move || {
        let peer_addr = stream.peer_addr();
        for message in write_rx {
            match message {
                WriteMessage::Broadcast { data } => match stream.write_all(data.as_bytes()) {
                    Ok(_) => println!("successfully wrote data to {:?}: {data}", peer_addr),
                    Err(err) => {
                        eprintln!("failed to write data to {:?}: {data}, {err}", peer_addr)
                    }
                },
                WriteMessage::ConnectionClosed => break,
            }
        }
    });
}

type ConnectionId = SocketAddr;

enum ReadMessage {
    Broadcast {
        connection_id: ConnectionId,
        data: String,
    },
    ConnectionClosed {
        connection_id: ConnectionId,
    },
}

enum WriteMessage {
    Broadcast { data: String },
    ConnectionClosed,
}
