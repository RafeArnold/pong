use std::{
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
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
    spawn(move || loop {
        println!("listening for incoming connections!");
        for stream in server.incoming() {
            match stream {
                Ok(stream) => handle_stream(stream),
                Err(err) => {
                    eprintln!("incoming connection failure: {err}")
                }
            }
        }
    })
}

fn handle_stream(mut stream: TcpStream) {
    spawn(move || {
        let peer_addr = stream.peer_addr();
        println!("connection established from {:?}", peer_addr);
        let mut reader = BufReader::new(
            stream
                .try_clone()
                .expect(&format!("failed to clone connection {:?}", peer_addr)),
        );
        let mut buffer = String::new();
        loop {
            match reader.read_line(&mut buffer) {
                Ok(n) => {
                    if n == 0 {
                        println!("connection {:?} closed", peer_addr);
                        break;
                    }
                    println!("successfully read data from {:?}: {buffer}", peer_addr);
                    match stream.write_all(buffer.as_bytes()) {
                        Ok(_) => println!("successfully wrote data to {:?}: {buffer}", peer_addr),
                        Err(err) => {
                            eprintln!("failed to write data to {:?}: {buffer}, {err}", peer_addr)
                        }
                    }
                }
                Err(err) => eprintln!("failed to read from {:?}: {err}", peer_addr),
            };
            buffer.clear();
        }
    });
}
