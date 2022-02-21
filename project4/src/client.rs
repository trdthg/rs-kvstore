use std::{io::{BufReader, BufWriter, Write}, net::{TcpStream, ToSocketAddrs}};

use serde::Deserialize;
use serde_json::{de::IoRead, Deserializer};

use crate::{Result, common::{Request, GetResponse, SetResponse, RemoveResponse}, KvsError};

pub struct KvsClient {
    reader: Deserializer<IoRead<BufReader<TcpStream>>>,
    writer: BufWriter<TcpStream>,
}

impl KvsClient {
    pub fn connect<A: ToSocketAddrs>(addr: A) -> Result<Self> {
        let tcp_reader = TcpStream::connect(addr)?;
        let tcp_writer = tcp_reader.try_clone()?;
        Ok(KvsClient {
            reader: Deserializer::from_reader(BufReader::new(tcp_reader)),
            writer: BufWriter::new(tcp_writer),
        })
    }

    pub fn get(&mut self, key: String) -> Result<Option<String>> {
        serde_json::to_writer(&mut self.writer, &Request::Get { key })?;
        self.writer.flush()?;
        match GetResponse::deserialize(&mut self.reader)? {
            GetResponse::Ok(value) => Ok(value),
            GetResponse::Err(e) => Err(KvsError::StringError(e))
        }
    }

    pub fn set(&mut self, key: String, value: String) -> Result<()> {
        serde_json::to_writer(&mut self.writer, &Request::Set { key, value })?;
        self.writer.flush()?;
        let resp = SetResponse::deserialize(&mut self.reader)?;
        match resp {
            SetResponse::Ok(_) => Ok(()),
            SetResponse::Err(e) => Err(KvsError::StringError(e)),
        }
    }

    pub fn remove(&mut self, key: String) -> Result<()> {
        serde_json::to_writer(&mut self.writer, &Request::Remove { key })?;
        self.writer.flush()?;
        match RemoveResponse::deserialize(&mut self.reader)? {
            RemoveResponse::Ok(_) => Ok(()),
            RemoveResponse::Err(e) => Err(KvsError::StringError(e)),
        }
    }
}