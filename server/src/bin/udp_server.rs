use std::net::UdpSocket;

fn main() {
    let server = UdpSocket::bind("127.0.0.1:8080").expect("failed to start udp server");
    let mut buf = [0; 256];
    loop {
        match server.recv_from(&mut buf) {
            Ok((n, addr)) => {
                let n = deserialize_u64(&buf[..n]);
                server.send_to(&serialize_u64(n + 1), addr).unwrap();
            }
            Err(err) => eprintln!("failed to receive: {}", err),
        }
    }
}

fn deserialize_u64(u: &[u8]) -> u64 {
    u.into_iter()
        .enumerate()
        .fold(0, |acc, (idx, n)| acc | ((*n as u64) << (8 * idx)))
}

fn serialize_u64(u: u64) -> [u8; 8] {
    std::array::from_fn(|idx| (u >> 8 * idx) as u8)
}
