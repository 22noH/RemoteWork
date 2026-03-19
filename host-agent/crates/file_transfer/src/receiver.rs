use anyhow::Result;
use std::path::Path;
use tokio::io::AsyncWriteExt;
use sha2::{Sha256, Digest};
use std::collections::HashMap;

pub struct FileReceiver {
    transfers: HashMap<String, ReceivingTransfer>,
}

struct ReceivingTransfer {
    file: tokio::fs::File,
    expected_hash: String,
    hasher: Sha256,
    bytes_received: u64,
}

impl FileReceiver {
    pub fn new() -> Self {
        Self { transfers: HashMap::new() }
    }

    pub async fn start_receive(
        &mut self,
        transfer_id: String,
        file_path: impl AsRef<Path>,
        expected_hash: String,
    ) -> Result<()> {
        let file = tokio::fs::File::create(file_path).await?;
        self.transfers.insert(
            transfer_id,
            ReceivingTransfer {
                file,
                expected_hash,
                hasher: Sha256::new(),
                bytes_received: 0,
            },
        );
        Ok(())
    }

    /// Returns the verified hash if this was the last chunk, None otherwise.
    pub async fn receive_chunk(
        &mut self,
        transfer_id: &str,
        data: &[u8],
        is_last: bool,
    ) -> Result<Option<String>> {
        let transfer = self
            .transfers
            .get_mut(transfer_id)
            .ok_or_else(|| anyhow::anyhow!("Unknown transfer: {}", transfer_id))?;

        transfer.file.write_all(data).await?;
        transfer.hasher.update(data);
        transfer.bytes_received += data.len() as u64;

        if is_last {
            let hash = hex::encode(transfer.hasher.clone().finalize());
            let expected = transfer.expected_hash.clone();
            self.transfers.remove(transfer_id);

            if hash != expected {
                anyhow::bail!("Hash mismatch: expected {}, got {}", expected, hash);
            }
            return Ok(Some(hash));
        }
        Ok(None)
    }
}
