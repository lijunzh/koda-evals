//! Append-only journal of accepted ids. A line is on disk before the
//! accept is acknowledged, so a restart rebuilds the dedupe set from it.

use std::io;
use std::path::Path;

use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncWriteExt;
use tokio::sync::{mpsc, oneshot};

type Entry = (String, oneshot::Sender<io::Result<()>>);

/// Handle to the writer task that owns the file.
pub struct Journal {
    tx: mpsc::Sender<Entry>,
}

impl Journal {
    /// Opens (or creates) the journal at `path`, returning its existing
    /// lines and a handle whose writer task appends to it.
    pub async fn open(path: &Path) -> io::Result<(Vec<String>, Self)> {
        let existing = match tokio::fs::read_to_string(path).await {
            Ok(text) => text.lines().map(str::to_owned).collect(),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(e),
        };
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await?;
        let (tx, mut rx) = mpsc::channel::<Entry>(64);
        tokio::spawn(async move {
            while let Some((line, ack)) = rx.recv().await {
                let _ = ack.send(write_line(&mut file, &line).await);
            }
        });
        Ok((existing, Self { tx }))
    }

    /// Appends one line and returns once it has reached the file.
    pub async fn append(&self, line: &str) -> io::Result<()> {
        let (ack_tx, ack_rx) = oneshot::channel();
        self.tx
            .send((line.to_owned(), ack_tx))
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "journal writer stopped"))?;
        ack_rx
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "journal writer stopped"))?
    }
}

async fn write_line(file: &mut File, line: &str) -> io::Result<()> {
    file.write_all(line.as_bytes()).await?;
    file.write_all(b"\n").await?;
    file.flush().await
}
