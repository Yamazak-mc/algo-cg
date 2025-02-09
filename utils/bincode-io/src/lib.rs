use serde::{de::DeserializeOwned, Serialize};
use std::{collections::VecDeque, future::Future, marker::PhantomData};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use tracing::info;

pub trait ReadBincodeExt {
    fn try_read_bincode<T: DeserializeOwned>(
        &mut self,
        reader: &mut BincodeReader<T>,
    ) -> Result<(), TryReadBincodeError>;
}

impl ReadBincodeExt for TcpStream {
    fn try_read_bincode<T: DeserializeOwned>(
        &mut self,
        reader: &mut BincodeReader<T>,
    ) -> Result<(), TryReadBincodeError> {
        reader.try_read(self)
    }
}

#[derive(Debug, Clone)]
pub struct BincodeReader<T: DeserializeOwned> {
    bytes: Vec<u8>,
    pub data: VecDeque<T>,
}

impl<T: DeserializeOwned> BincodeReader<T> {
    pub fn new(buf_size: usize) -> Self {
        Self {
            bytes: vec![0; buf_size],
            data: VecDeque::new(),
        }
    }

    fn try_read(&mut self, stream: &mut TcpStream) -> Result<(), TryReadBincodeError> {
        match stream.try_read(&mut self.bytes) {
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                Err(TryReadBincodeError::WouldBlock)
            }
            Err(e) => Err(TryReadBincodeError::Other(e.into())),
            Ok(0) => Err(TryReadBincodeError::Read0Bytes),
            Ok(n) => {
                self.process_read_bytes(n)?;
                Ok(())
            }
        }
    }

    fn process_read_bytes(&mut self, n: usize) -> anyhow::Result<()> {
        let mut i = 0;
        while i < n {
            let len = u32::from_be_bytes(self.bytes[i..i + 4].try_into().unwrap()) as usize;

            i += 4;

            info!("deserializing bytes: range={}..{}", i, i + len);
            self.data
                .push_back(bincode::deserialize(&self.bytes[i..i + len])?);

            i += len;
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TryReadBincodeError {
    #[error("read would block")]
    WouldBlock,
    #[error("read 0 bytes")]
    Read0Bytes,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl TryReadBincodeError {
    pub fn would_block(&self) -> bool {
        matches!(self, Self::WouldBlock)
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct ReceivedSingle<T> {
    _len: u32,
    pub data: T,
}

pub trait SerializeBincodeExt: Serialize {
    fn to_bincode(&self) -> bincode::Result<Vec<u8>> {
        let data = bincode::serialize(self)?;

        let len = data.len();
        let mut ret = vec![0; 4 + len];
        ret[0..4].copy_from_slice(&(len as u32).to_be_bytes());
        ret[4..].copy_from_slice(&data);

        Ok(ret)
    }
}

impl<T: Serialize> SerializeBincodeExt for T {}

pub struct TcpStreamWrapper<I: DeserializeOwned, O: Serialize> {
    stream: TcpStream,
    reader: BincodeReader<I>,
    _marker: PhantomData<fn(&O)>,
}

impl<I: DeserializeOwned, O: Serialize> TcpStreamWrapper<I, O> {
    pub fn new(stream: TcpStream, buf_size: usize) -> Self {
        Self {
            stream,
            reader: BincodeReader::new(buf_size),
            _marker: PhantomData,
        }
    }

    pub async fn read(&mut self) -> anyhow::Result<Option<I>> {
        if let Some(data) = self.reader.data.pop_front() {
            return Ok(Some(data));
        }

        let n = self.stream.read(&mut self.reader.bytes).await?;
        self.reader.process_read_bytes(n)?;

        Ok(self.reader.data.pop_front())
    }

    pub async fn readable(&self) -> Result<(), std::io::Error> {
        if !self.reader.data.is_empty() {
            return Ok(());
        }

        self.stream.readable().await
    }

    pub fn try_read(&mut self) -> Result<I, TryReadBincodeError> {
        if let Some(data) = self.reader.data.pop_front() {
            return Ok(data);
        }

        self.stream.try_read_bincode(&mut self.reader)?;
        Ok(self.reader.data.pop_front().unwrap())
    }

    pub async fn write(&mut self, message: &O) -> anyhow::Result<()> {
        let msg = message.to_bincode()?;

        self.stream.write_all(&msg).await?;
        Ok(())
    }

    pub async fn writable(&self) -> impl Future<Output = Result<(), std::io::Error>> + '_ {
        self.stream.writable()
    }

    pub fn try_write(&self, message: &O) -> Result<(), TryWriteBincodeError> {
        let msg = message
            .to_bincode()
            .map_err(|e| TryWriteBincodeError::Other(e.into()))?;

        match self.stream.try_write(&msg) {
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                Err(TryWriteBincodeError::WouldBlock)
            }
            Err(e) => Err(TryWriteBincodeError::Other(e.into())),
            Ok(n) => {
                debug_assert_eq!(n, msg.len());
                Ok(())
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TryWriteBincodeError {
    #[error("write would block")]
    WouldBlock,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
