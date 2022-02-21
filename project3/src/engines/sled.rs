
use sled::{Db, Tree};

use crate::{KvsEngine, Result, KvsError};

pub struct SledKvsEngine(Db);

impl SledKvsEngine {
    pub fn new(db: Db) -> Self {
        SledKvsEngine(db)
    }
}

impl KvsEngine for SledKvsEngine {
    fn set(&mut self, key: String, value: String) -> Result<()> {
        let tree: &Tree = &self.0;
        tree.insert(key, value.into_bytes()).map(|_| ())?;
        tree.flush()?;
        Ok(())
    }

    fn get(&mut self, key: String) -> Result<Option<String>> {
        let tree: &Tree = &self.0;
        Ok(tree.get(key)?
            .map(|i_vec| AsRef::<[u8]>::as_ref(&i_vec).to_vec())
            // .map(|i_vec| i_vec.to_vec())
            .map(|x| String::from_utf8(x))
            .transpose()?
        )
    }

    fn remove(&mut self, key: String) -> Result<()> {
        let tree: &Tree = &self.0;
        tree.remove(key)?.ok_or(KvsError::KeyNotFound)?;
        tree.flush()?;
        Ok(())
    }
}