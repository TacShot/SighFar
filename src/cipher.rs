use anyhow::{Result, bail};
use sha2::{Digest, Sha256, Sha512};

use crate::models::{OperationKind, TechniqueDescriptor};

#[derive(Default)]
pub struct CipherPipeline;

impl CipherPipeline {
    pub fn encode(&self, input: &str, techniques: &[TechniqueDescriptor]) -> Result<String> {
        techniques.iter().try_fold(input.to_string(), |partial, technique| {
            self.transform(&partial, technique, OperationKind::Encode)
        })
    }

    pub fn decode(&self, input: &str, techniques: &[TechniqueDescriptor]) -> Result<String> {
        techniques.iter().rev().try_fold(input.to_string(), |partial, technique| {
            self.transform(&partial, technique, OperationKind::Decode)
        })
    }

    fn transform(
        &self,
        input: &str,
        technique: &TechniqueDescriptor,
        direction: OperationKind,
    ) -> Result<String> {
        match technique {
            TechniqueDescriptor::Morse => process_morse(input, direction),
            TechniqueDescriptor::Caesar { shift } => Ok(process_caesar(input, *shift, direction)),
            TechniqueDescriptor::Vigenere { keyword } => process_vigenere(input, keyword, direction),
            TechniqueDescriptor::RailFence { rails } => process_rail_fence(input, *rails, direction),
            TechniqueDescriptor::Reverse => Ok(input.chars().rev().collect()),
            TechniqueDescriptor::Sha256 => process_sha256(input, direction),
            TechniqueDescriptor::Sha512 => process_sha512(input, direction),
        }
    }
}

fn process_sha256(input: &str, direction: OperationKind) -> Result<String> {
    match direction {
        OperationKind::Encode => {
            let mut hasher = Sha256::new();
            hasher.update(input.as_bytes());
            Ok(hex::encode(hasher.finalize()))
        }
        OperationKind::Decode => {
            bail!("SHA-256 is a one-way hash function and cannot be decoded.")
        }
    }
}

fn process_sha512(input: &str, direction: OperationKind) -> Result<String> {
    match direction {
        OperationKind::Encode => {
            let mut hasher = Sha512::new();
            hasher.update(input.as_bytes());
            Ok(hex::encode(hasher.finalize()))
        }
        OperationKind::Decode => {
            bail!("SHA-512 is a one-way hash function and cannot be decoded.")
        }
    }
}

fn process_morse(input: &str, direction: OperationKind) -> Result<String> {
    let map = morse_map();
    match direction {
        OperationKind::Encode => {
            let mut parts = Vec::new();
            for ch in input.to_lowercase().chars() {
                let symbol = map
                    .iter()
                    .find(|(plain, _)| *plain == ch)
                    .map(|(_, code)| *code)
                    .ok_or_else(|| anyhow::anyhow!("Unsupported Morse character: {ch}"))?;
                parts.push(symbol.to_string());
            }
            Ok(parts.join(" "))
        }
        OperationKind::Decode => {
            let mut output = String::new();
            for token in input.split_whitespace() {
                let ch = map
                    .iter()
                    .find(|(_, code)| *code == token)
                    .map(|(plain, _)| *plain)
                    .ok_or_else(|| anyhow::anyhow!("Unsupported Morse token: {token}"))?;
                output.push(ch);
            }
            Ok(output)
        }
    }
}

fn process_caesar(input: &str, shift: i32, direction: OperationKind) -> String {
    let applied = match direction {
        OperationKind::Encode => shift,
        OperationKind::Decode => -shift,
    };

    input.chars().map(|ch| rotate_ascii(ch, applied)).collect()
}

fn process_vigenere(input: &str, keyword: &str, direction: OperationKind) -> Result<String> {
    let key: Vec<i32> = keyword
        .chars()
        .filter(|ch| ch.is_ascii_alphabetic())
        .map(|ch| ch.to_ascii_lowercase() as i32 - 'a' as i32)
        .collect();

    if key.is_empty() {
        bail!("Vigenere keyword must include letters.");
    }

    let mut index = 0usize;
    let output = input
        .chars()
        .map(|ch| {
            if !ch.is_ascii_alphabetic() {
                return ch;
            }

            let shift = key[index % key.len()];
            index += 1;
            let applied = match direction {
                OperationKind::Encode => shift,
                OperationKind::Decode => -shift,
            };
            rotate_ascii(ch, applied)
        })
        .collect();

    Ok(output)
}

fn process_rail_fence(input: &str, rails: usize, direction: OperationKind) -> Result<String> {
    if rails < 2 {
        bail!("RailFence requires at least 2 rails.");
    }

    match direction {
        OperationKind::Encode => Ok(rail_fence_encode(input, rails)),
        OperationKind::Decode => Ok(rail_fence_decode(input, rails)),
    }
}

fn rail_fence_encode(input: &str, rails: usize) -> String {
    if input.chars().count() <= 1 {
        return input.to_string();
    }

    let mut fence = vec![String::new(); rails];
    let mut rail = 0isize;
    let mut direction = 1isize;

    for ch in input.chars() {
        fence[rail as usize].push(ch);
        if rail == 0 {
            direction = 1;
        } else if rail == rails as isize - 1 {
            direction = -1;
        }
        rail += direction;
    }

    fence.concat()
}

fn rail_fence_decode(input: &str, rails: usize) -> String {
    if input.chars().count() <= 1 {
        return input.to_string();
    }

    let pattern = rail_pattern(input.chars().count(), rails);
    let mut counts = vec![0usize; rails];
    for rail in &pattern {
        counts[*rail] += 1;
    }

    let chars: Vec<char> = input.chars().collect();
    let mut rails_data: Vec<Vec<char>> = Vec::with_capacity(rails);
    let mut cursor = 0usize;
    for count in counts {
        rails_data.push(chars[cursor..cursor + count].to_vec());
        cursor += count;
    }

    let mut offsets = vec![0usize; rails];
    let mut output = String::with_capacity(chars.len());
    for rail in pattern {
        output.push(rails_data[rail][offsets[rail]]);
        offsets[rail] += 1;
    }
    output
}

fn rail_pattern(length: usize, rails: usize) -> Vec<usize> {
    let mut rail = 0isize;
    let mut direction = 1isize;
    let mut pattern = Vec::with_capacity(length);

    for _ in 0..length {
        pattern.push(rail as usize);
        if rail == 0 {
            direction = 1;
        } else if rail == rails as isize - 1 {
            direction = -1;
        }
        rail += direction;
    }

    pattern
}

fn rotate_ascii(ch: char, amount: i32) -> char {
    let rotate_from = |base: u8, letter: char| -> char {
        let offset = letter as i32 - base as i32;
        let rotated = ((offset + amount) % 26 + 26) % 26;
        (base + rotated as u8) as char
    };

    match ch {
        'A'..='Z' => rotate_from(b'A', ch),
        'a'..='z' => rotate_from(b'a', ch),
        _ => ch,
    }
}

fn morse_map() -> &'static [(char, &'static str)] {
    &[
        ('a', ".-"), ('b', "-..."), ('c', "-.-."), ('d', "-.."), ('e', "."),
        ('f', "..-."), ('g', "--."), ('h', "...."), ('i', ".."), ('j', ".---"),
        ('k', "-.-"), ('l', ".-.."), ('m', "--"), ('n', "-."), ('o', "---"),
        ('p', ".--."), ('q', "--.-"), ('r', ".-."), ('s', "..."), ('t', "-"),
        ('u', "..-"), ('v', "...-"), ('w', ".--"), ('x', "-..-"), ('y', "-.--"),
        ('z', "--.."), ('0', "-----"), ('1', ".----"), ('2', "..---"),
        ('3', "...--"), ('4', "....-"), ('5', "....."), ('6', "-...."),
        ('7', "--..."), ('8', "---.."), ('9', "----."), (' ', "/"),
    ]
}

#[cfg(test)]
mod tests {
    use super::CipherPipeline;
    use crate::models::TechniqueDescriptor;

    #[test]
    fn cipher_pipeline_round_trip() {
        let pipeline = CipherPipeline;
        let techniques = vec![
            TechniqueDescriptor::Caesar { shift: 4 },
            TechniqueDescriptor::Reverse,
            TechniqueDescriptor::Vigenere {
                keyword: "smile".to_string(),
            },
            TechniqueDescriptor::RailFence { rails: 3 },
        ];

        let encoded = pipeline.encode("Secret Message", &techniques).unwrap();
        let decoded = pipeline.decode(&encoded, &techniques).unwrap();

        assert_eq!(decoded, "Secret Message");
    }

    #[test]
    fn morse_round_trip() {
        let pipeline = CipherPipeline;

        let encoded = pipeline
            .encode("sos 2", &[TechniqueDescriptor::Morse])
            .unwrap();
        let decoded = pipeline
            .decode(&encoded, &[TechniqueDescriptor::Morse])
            .unwrap();

        assert_eq!(encoded, "... --- ... / ..---");
        assert_eq!(decoded, "sos 2");
    }
}
