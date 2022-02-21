use crate::Alphabet;

const UPPERCASEOFFSET: i8 = 65;
const LOWERCASEOFFSET: i8 = 71;
const DIGITOFFSET: i8 = -4;

pub struct Classic;

impl Alphabet for Classic {
    fn ger_char_from_index(&self, index: u8) -> Option<char> {
        let index = index as i8;

        // 这一布是把用户的0-64给映射到ascii对应的编码
        let ascii_index = match index {
            0..=25 => index + UPPERCASEOFFSET,  // 大写 65 - 0 = 65
            26..=51 => index + LOWERCASEOFFSET,  // 小写 97 - 26 = 71
            52..=61 => index + DIGITOFFSET,  // 数字 48 - 52 = -4
            62 => 43,  // + 43
            63 => 47,  // / 47
            _ => return None,
        };

        return Some(ascii_index as u8 as char);
    }

    fn get_index_from_char(&self, character: char) -> Option<u8> {
        // let character = character as i8;
        let base64_index = match character {
            'A'..='Z' => character as i8 - UPPERCASEOFFSET,
            'a'..='z' => character as i8 - LOWERCASEOFFSET,
            '0'..='9' => character as i8 - DIGITOFFSET,
            '+' => 62,
            '/' => 63,
            _ => return None,
        };

        Some(base64_index as u8)
    }

    fn get_padding_char(&self) -> char {
        '='
    }
}

#[cfg(test)]
mod test {
    use super::*;


    #[test]
    fn get_char_from_index() {
        let alphbats = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/".chars();
        let classic = Classic;
        for (char, i) in alphbats.zip(0..64) {
            assert_eq!(classic.ger_char_from_index(i), Some(char));
        }
        assert_eq!(classic.ger_char_from_index(64), None);
        assert_eq!(classic.ger_char_from_index(65), None);
        assert_eq!(classic.ger_char_from_index(127), None);
        assert_eq!(classic.ger_char_from_index(128), None);
        assert_eq!(classic.ger_char_from_index(255), None);
    }

    #[test]
    fn get_index_from_char() {
        let alphbats = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/".chars();
        let classic = Classic;
        for (char, i) in alphbats.zip(0..64) {
            assert_eq!(classic.get_index_from_char(char), Some(i));
        }
        for char in "<>?,.:\";{}[]|'!@#$%^&*()_-=\\~`".chars() {
            assert_eq!(classic.get_index_from_char(char), None);
        }
    }

    #[test]
    fn get_padding_char() {
        let classic = Classic;
        assert_eq!(classic.get_padding_char(), '=');
    }
}