mod error;
mod kv;
pub use error::{KvsError, Result};
pub use kv::{KvStore};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
