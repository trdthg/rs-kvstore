use crate::{Alphabet, Classic};

fn split(chunk: &[u8]) -> Vec<u8> {
    match chunk.len() {
        // 不足的都是末尾补0
        1 => vec![
            &chunk[0] >> 2, // 第一个前6位
            (&chunk[0] & 0b00000011) << 4,  // 后两位+0000
        ],
        2 => vec![
            &chunk[0] >> 2,
            (&chunk[0] & 0b00000011) << 4 | &chunk[1] >> 4,
            (&chunk[1] & 0b00001111) << 2,
        ],
        // 3*8个刚好分为4*6
        3 => vec![
            &chunk[0] >> 2, //第一个右移两位 00xxxxxx
            (&chunk[0] & 0b00000011) << 4 | &chunk[1] >> 4,  // 第一个后2位 + 第二个前4位
            (&chunk[1] & 0b00001111) << 2 | &chunk[2] >> 6,  // 第二个后4位 + 第三个前2位
            &chunk[2] & 0b00111111  // 第三个后6位
        ],
        _ => unreachable!()
    }
}

pub fn encode(data: &[u8]) -> String {
    let classic_alphabet = &Classic {};
    encode_using_alphabet(classic_alphabet, data)
}


fn encode_using_alphabet<T: Alphabet>(alphabet: &T, data: &[u8]) -> String {
    // data.chunks(3).map(split).collect().to_string()
    let encoded = data.chunks(3).map(split).flat_map(|chunk| {
        encode_chunk(alphabet, chunk)
    });

    String::from_iter(encoded)
}

fn encode_chunk<T: Alphabet>(alphabet: &T, chunk: Vec<u8>) -> Vec<char> {
    let mut out = vec![alphabet.get_padding_char(); 4];

    for i in 0..chunk.len() {
        if let Some(char) = alphabet.ger_char_from_index(chunk[i]) {
            out[i] = char;
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::encode;

    #[test]
    fn test_single_char() {
        let input_str = "a";
        let expected = "YQ==";

        let input_data = input_str.as_bytes();

        assert_eq!(encode(input_data), expected);
    }

    #[test]
    fn test_two_chars() {
        let input_str = "ab";
        let expected = "YWI=";

        let input_data = input_str.as_bytes();

        assert_eq!(encode(input_data), expected);
    }

    #[test]
    fn test_three_chars() {
        let input_str = "abc";
        let expected = "YWJj";

        let input_data = input_str.as_bytes();

        assert_eq!(encode(input_data), expected);
    }

    #[test]
    fn tests_short_string() {
        let input_str = "Hello, world!";
        let expected = "SGVsbG8sIHdvcmxkIQ==";

        let input = input_str.as_bytes();

        assert_eq!(encode(input), expected);
    }

    #[test]
    fn test_longer_string() {
        let input_str = "And here be a bit longer text. Let's see how it goes!";
        let expected = "QW5kIGhlcmUgYmUgYSBiaXQgbG9uZ2VyIHRleHQuIExldCdzIHNlZSBob3cgaXQgZ29lcyE=";

        let input_data = input_str.as_bytes();

        assert_eq!(encode(input_data), expected);
    }
}