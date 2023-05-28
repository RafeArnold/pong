use std::{
    io::{stdin, Read, Write},
    net::TcpStream,
    thread::spawn,
};

fn main() {
    let server_addr = "127.0.0.1:8080";
    let client = TcpStream::connect(server_addr).expect("failed to connect to server");
    println!("connected to {server_addr}");
    handle_stdin(client.try_clone().expect("failed to clone connection"));
    handle_incoming(client);
}

fn handle_stdin(mut client: TcpStream) {
    spawn(move || {
        let stdin = stdin();
        for line in stdin.lines() {
            match line {
                Ok(mut line) => {
                    println!("sending to server: {line}");
                    line.push('\n');
                    match client.write_all(line.as_bytes()) {
                        Ok(_) => println!("successfully wrote to server: {line}"),
                        Err(err) => eprintln!("failed to write to server: {line}, {err}"),
                    }
                }
                Err(err) => eprintln!("failed to read line from stdin: {err}"),
            }
        }
    });
}

fn handle_incoming(mut client: TcpStream) {
    let mut buffer = [0; 256];
    loop {
        match client.read(&mut buffer) {
            Ok(n) => {
                if n == 0 {
                    println!("server closed connection");
                    break;
                }
                let text = std::str::from_utf8(&buffer[..n]).ok();
                println!("successfully read data from server: {:?}", text);
            }
            Err(err) => eprintln!("failed to read from server: {err}"),
        };
    }
}
