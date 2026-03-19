use anyhow::Result;
use std::path::Path;
use tokio::io::AsyncReadExt;
use sha2::{Sha256, Digest};
use prost::Message;
use proto::remote_work::{FileChunk, FileTransferMessage, FileTransferRequest, file_transfer_message::Payload};
use uuid::Uuid;

const CHUNK_SIZE: usize = 65_536; // 64 KB

pub struct FileSender;

impl FileSender {
    pub async fn prepare_request(
        file_path: &Path,
        dest_path: &str,
    ) -> Result<FileTransferRequest> {
        let metadata = tokio::fs::metadata(file_path).await?;
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Compute SHA-256 for integrity verification
        let mut file = tokio::fs::File::open(file_path).await?;
        let mut hasher = Sha256::new();
        let mut buf = vec![0u8; CHUNK_SIZE];
        loop {
            let n = file.read(&mut buf).await?;
            if n == 0 { break; }
            hasher.update(&buf[..n]);
        }
        let hash = hex::encode(hasher.finalize());

        Ok(FileTransferRequest {
            transfer_id: Uuid::new_v4().to_string(),
            file_name,
            file_size: metadata.len(),
            sha256_hash: hash,
            destination_path: dest_path.to_string(),
        })
    }

    pub async fn send_chunks(
        file_path: &Path,
        transfer_id: &str,
        mut send_fn: impl FnMut(Vec<u8>) -> Result<()>,
    ) -> Result<()> {
        let mut file = tokio::fs::File::open(file_path).await?;
        let mut buf = vec![0u8; CHUNK_SIZE];
        let mut offset = 0u64;

        loop {
            let n = file.read(&mut buf).await?;
            if n == 0 { break; }

            let last_chunk = n < CHUNK_SIZE;
            let msg = FileTransferMessage {
                payload: Some(Payload::Chunk(FileChunk {
                    transfer_id: transfer_id.to_string(),
                    offset,
                    data: buf[..n].to_vec(),
                    last_chunk,
                })),
            };

            send_fn(msg.encode_to_vec())?;
            offset += n as u64;

            if last_chunk { break; }
        }
        Ok(())
    }
}
