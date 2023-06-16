use std::net::UdpSocket;

fn main() {
    let client = UdpSocket::bind("127.0.0.1:0").unwrap();
    client
        .connect("127.0.0.1:8080")
        .expect("failed to connect to udp server");
    client
        .send(&[0; 8])
        .expect("failed to send data to udp server");
    let mut buf = [0; 8];
    loop {
        match client.recv(&mut buf) {
            Ok(n) => {
                let buf = &buf[..n];
                let n = deserialize_u64(buf);
                if n >= 10000 {
                    break;
                }
                client.send(&serialize_u64(n)).unwrap();
            }
            Err(err) => eprintln!("failed to receive data from udp server: {}", err),
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

#[cfg(test)]
mod tests {
    use crate::{deserialize_u64, serialize_u64};

    #[test]
    fn deserialize() {
        assert_eq!(
            deserialize_u64(&[0b10101010, 0b11111111, 0b00000000, 0b11011011]),
            0b11011011000000001111111110101010
        );
        assert_eq!(
            deserialize_u64(&[
                0b10010010, 0b00011110, 0b11101111, 0b00111000, 0b10101010, 0b11111111, 0b00000000,
                0b11011011
            ]),
            0b1101101100000000111111111010101000111000111011110001111010010010
        );
    }

    #[test]
    fn serialize() {
        assert_eq!(
            serialize_u64(0b11011011000000001111111110101010),
            [0b10101010, 0b11111111, 0b00000000, 0b11011011, 0, 0, 0, 0]
        );
        assert_eq!(
            serialize_u64(0b1101101100000000111111111010101000111000111011110001111010010010),
            [
                0b10010010, 0b00011110, 0b11101111, 0b00111000, 0b10101010, 0b11111111, 0b00000000,
                0b11011011
            ]
        );
    }
}
