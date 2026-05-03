//! 8-char base62 short identifier for photo permalinks.
//! /u/<handle>/p/<short_id>. ~2.18*10^14 keyspace.

use nanoid::nanoid;

const ALPHABET: [char; 62] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I',
    'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', 'a', 'b',
    'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u',
    'v', 'w', 'x', 'y', 'z',
];

pub fn generate() -> String {
    nanoid!(8, &ALPHABET)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn produces_8_chars() {
        let s = generate();
        assert_eq!(s.chars().count(), 8);
    }

    #[test]
    fn alphabet_is_base62() {
        for _ in 0..1000 {
            let s = generate();
            for c in s.chars() {
                assert!(c.is_ascii_alphanumeric(), "char {c} not base62");
            }
        }
    }

    #[test]
    fn collisions_unlikely_in_small_set() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for _ in 0..10_000 {
            assert!(set.insert(generate()));
        }
    }
}
