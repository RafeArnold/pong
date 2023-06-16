use aes::Aes256;
use fpe::ff1::{FlexibleNumeralString, FF1};
use shared::LOBBY_ID_LEN;

const LOBBY_ID_RADIX: u32 = 32;

pub struct LobbyIdGenerator {
    id_count: u32,
    ff1: FF1<Aes256>,
}

impl LobbyIdGenerator {
    pub fn new(key: &[u8; 32]) -> Self {
        Self {
            id_count: 0,
            ff1: FF1::<Aes256>::new(key, LOBBY_ID_RADIX as u32).unwrap(),
        }
    }

    pub fn next_id(&mut self) -> String {
        let num_str = FlexibleNumeralString::from(
            (0..LOBBY_ID_LEN)
                .map(|idx| ((self.id_count >> 5 * idx) as u16) & 0b11111)
                .collect::<Vec<_>>(),
        );
        self.id_count = self.id_count.wrapping_add(1);
        let lobby_id = self.ff1.encrypt(&[], &num_str).unwrap();
        String::from_utf8(
            Vec::<u16>::from(lobby_id)
                .into_iter()
                .map(|n| match n as u8 {
                    n if n < 8 => n + b'2', // don't use 0 and 1 in IDs because they could be mixed up with O and I, plus it allows the use of the entire alphabet (A..Z).
                    n => n + b'A' - 8,
                })
                .collect(),
        )
        .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use crate::lobby_id_generator::LobbyIdGenerator;

    #[test]
    fn distinct_by_key() {
        let key = [0; 32];
        let mut lobby_id_generator = LobbyIdGenerator::new(&key);
        assert_eq!(lobby_id_generator.next_id(), "H5MS");
        assert_eq!(lobby_id_generator.next_id(), "EK9F");
        assert_eq!(lobby_id_generator.next_id(), "FWSI");
        assert_eq!(lobby_id_generator.next_id(), "5B4M");
        let key = [1; 32];
        let mut lobby_id_generator = LobbyIdGenerator::new(&key);
        assert_eq!(lobby_id_generator.next_id(), "B4RL");
        assert_eq!(lobby_id_generator.next_id(), "X9UE");
        assert_eq!(lobby_id_generator.next_id(), "2E9J");
        assert_eq!(lobby_id_generator.next_id(), "ELIX");
    }
}

/// these tests take several seconds to run.
#[cfg(test)]
#[cfg(not(debug_assertions))]
mod release_tests {
    use std::collections::HashSet;

    use crate::lobby_id_generator::{LobbyIdGenerator, LOBBY_ID_LEN, LOBBY_ID_RADIX};

    #[test]
    fn uniqueness() {
        let key = [0; 32];
        let mut lobby_id_generator = LobbyIdGenerator::new(&key);
        let count = LOBBY_ID_RADIX.pow(LOBBY_ID_LEN as u32);
        let set = (0..count)
            .map(|_| lobby_id_generator.next_id())
            .collect::<HashSet<_>>();
        assert_eq!(set.len(), count as usize);
    }

    #[test]
    fn wrap_around() {
        let key = [0; 32];
        let mut lobby_id_generator = LobbyIdGenerator::new(&key);
        let first_id = lobby_id_generator.next_id();
        for _ in 0..LOBBY_ID_RADIX.pow(LOBBY_ID_LEN as u32) - 1 {
            lobby_id_generator.next_id();
        }
        assert_eq!(lobby_id_generator.next_id(), first_id);
    }
}
