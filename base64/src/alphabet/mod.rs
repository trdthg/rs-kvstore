mod classic;

pub use classic::Classic;

/// 没想到，项目一开始就奔着configurable去的
///

pub trait Alphabet {

    fn ger_char_from_index(&self, index: u8) -> Option<char>;

    fn get_index_from_char(&self, character: char) -> Option<u8>;

    fn get_padding_char(&self) -> char;

}
