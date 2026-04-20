use anyhow::{Result, bail};

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

    #[test]
    fn caesar_encode_preserves_case_and_non_alpha() {
        let pipeline = CipherPipeline;
        let techniques = &[TechniqueDescriptor::Caesar { shift: 3 }];
        let encoded = pipeline.encode("Hello, World!", techniques).unwrap();
        assert_eq!(encoded, "Khoor, Zruog!");
    }

    #[test]
    fn caesar_decode_reverses_encode() {
        let pipeline = CipherPipeline;
        let techniques = &[TechniqueDescriptor::Caesar { shift: 13 }];
        let encoded = pipeline.encode("The quick brown fox", techniques).unwrap();
        let decoded = pipeline.decode(&encoded, techniques).unwrap();
        assert_eq!(decoded, "The quick brown fox");
    }

    #[test]
    fn caesar_wrap_around() {
        let pipeline = CipherPipeline;
        let techniques = &[TechniqueDescriptor::Caesar { shift: 3 }];
        let encoded = pipeline.encode("xyz XYZ", techniques).unwrap();
        assert_eq!(encoded, "abc ABC");
    }

    #[test]
    fn caesar_shift_zero_is_identity() {
        let pipeline = CipherPipeline;
        let techniques = &[TechniqueDescriptor::Caesar { shift: 0 }];
        let text = "No Change Here 123!";
        let encoded = pipeline.encode(text, techniques).unwrap();
        assert_eq!(encoded, text);
    }

    #[test]
    fn caesar_shift_26_is_identity() {
        let pipeline = CipherPipeline;
        let techniques = &[TechniqueDescriptor::Caesar { shift: 26 }];
        let text = "Full Cycle";
        let encoded = pipeline.encode(text, techniques).unwrap();
        assert_eq!(encoded, text);
    }

    #[test]
    fn vigenere_encode_and_decode() {
        let pipeline = CipherPipeline;
        let techniques = &[TechniqueDescriptor::Vigenere {
            keyword: "key".to_string(),
        }];
        let encoded = pipeline.encode("Attack at dawn", techniques).unwrap();
        let decoded = pipeline.decode(&encoded, techniques).unwrap();
        assert_eq!(decoded, "Attack at dawn");
    }

    #[test]
    fn vigenere_non_alpha_passthrough() {
        let pipeline = CipherPipeline;
        let techniques = &[TechniqueDescriptor::Vigenere {
            keyword: "abc".to_string(),
        }];
        let encoded = pipeline.encode("Hello, 42!", techniques).unwrap();
        assert!(encoded.contains(", "));
        assert!(encoded.contains("42!"));
    }

    #[test]
    fn vigenere_empty_keyword_fails() {
        let pipeline = CipherPipeline;
        let techniques = &[TechniqueDescriptor::Vigenere {
            keyword: "123".to_string(),
        }];
        let result = pipeline.encode("test", techniques);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("keyword"));
    }

    #[test]
    fn rail_fence_encode_and_decode() {
        let pipeline = CipherPipeline;
        let techniques = &[TechniqueDescriptor::RailFence { rails: 3 }];
        let encoded = pipeline.encode("WEAREDISCOVEREDRUNATONCE", techniques).unwrap();
        let decoded = pipeline.decode(&encoded, techniques).unwrap();
        assert_eq!(decoded, "WEAREDISCOVEREDRUNATONCE");
    }

    #[test]
    fn rail_fence_two_rails() {
        let pipeline = CipherPipeline;
        let techniques = &[TechniqueDescriptor::RailFence { rails: 2 }];
        let text = "abcdef";
        let encoded = pipeline.encode(text, techniques).unwrap();
        let decoded = pipeline.decode(&encoded, techniques).unwrap();
        assert_eq!(decoded, text);
    }

    #[test]
    fn rail_fence_single_rail_fails() {
        let pipeline = CipherPipeline;
        let techniques = &[TechniqueDescriptor::RailFence { rails: 1 }];
        let result = pipeline.encode("test", techniques);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("2 rails"));
    }

    #[test]
    fn rail_fence_single_char_passthrough() {
        let pipeline = CipherPipeline;
        let techniques = &[TechniqueDescriptor::RailFence { rails: 3 }];
        let encoded = pipeline.encode("X", techniques).unwrap();
        assert_eq!(encoded, "X");
        let decoded = pipeline.decode("X", techniques).unwrap();
        assert_eq!(decoded, "X");
    }

    #[test]
    fn reverse_is_own_inverse() {
        let pipeline = CipherPipeline;
        let techniques = &[TechniqueDescriptor::Reverse];
        let text = "Hello World";
        let encoded = pipeline.encode(text, techniques).unwrap();
        assert_eq!(encoded, "dlroW olleH");
        let decoded = pipeline.decode(&encoded, techniques).unwrap();
        assert_eq!(decoded, text);
    }

    #[test]
    fn reverse_unicode_characters() {
        let pipeline = CipherPipeline;
        let techniques = &[TechniqueDescriptor::Reverse];
        // Test with actual multi-byte Unicode characters
        let encoded = pipeline.encode("héllo", techniques).unwrap();
        assert_eq!(encoded, "olléh");
    }

    #[test]
    fn morse_unsupported_char_encode_fails() {
        let pipeline = CipherPipeline;
        let result = pipeline.encode("hello@world", &[TechniqueDescriptor::Morse]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported Morse character"));
    }

    #[test]
    fn morse_unsupported_token_decode_fails() {
        let pipeline = CipherPipeline;
        let result = pipeline.decode("... BADTOKEN ---", &[TechniqueDescriptor::Morse]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported Morse token"));
    }

    #[test]
    fn pipeline_with_empty_techniques_is_identity() {
        let pipeline = CipherPipeline;
        let text = "unchanged";
        let encoded = pipeline.encode(text, &[]).unwrap();
        assert_eq!(encoded, text);
        let decoded = pipeline.decode(text, &[]).unwrap();
        assert_eq!(decoded, text);
    }

    #[test]
    fn morse_all_digits_round_trip() {
        let pipeline = CipherPipeline;
        let techniques = &[TechniqueDescriptor::Morse];
        let text = "0123456789";
        let encoded = pipeline.encode(text, techniques).unwrap();
        let decoded = pipeline.decode(&encoded, techniques).unwrap();
        assert_eq!(decoded, text);
    }

    #[test]
    fn morse_space_encodes_to_slash() {
        let pipeline = CipherPipeline;
        let encoded = pipeline.encode(" ", &[TechniqueDescriptor::Morse]).unwrap();
        assert_eq!(encoded, "/");
    }
}
