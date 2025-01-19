#![allow(async_fn_in_trait)]

use serde::Serialize;
use tokio::io::AsyncWriteExt;

pub trait AsyncWriteSerdeBincode: AsyncWriteExt {
    async fn write_bincode<'a, T>(&'a mut self, data: &T) -> anyhow::Result<()>
    where
        Self: Unpin,
        T: Serialize,
    {
        let data = bincode::serialize(data)?;
        self.write_all(&data).await?;
        Ok(())
    }
}

impl<T> AsyncWriteSerdeBincode for T where T: AsyncWriteExt {}
