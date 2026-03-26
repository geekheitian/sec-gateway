use chrono::Utc;
use std::path::{Path, PathBuf};
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RotationStrategy {
    Daily,
    Size,
}

pub struct FileAppender {
    base_path: PathBuf,
    max_size_bytes: u64,
    rotation_strategy: RotationStrategy,
    current_file: Mutex<Option<PathBuf>>,
}

impl FileAppender {
    pub fn new(
        base_path: impl AsRef<Path>,
        max_size_mb: u64,
        rotation_strategy: RotationStrategy,
    ) -> Self {
        Self {
            base_path: base_path.as_ref().to_path_buf(),
            max_size_bytes: max_size_mb * 1024 * 1024,
            rotation_strategy,
            current_file: Mutex::new(None),
        }
    }

    pub async fn append(&self, content: &str) -> Result<(), std::io::Error> {
        let target_file = self.get_target_file().await?;
        
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&target_file)
            .await?;

        file.write_all(content.as_bytes()).await?;
        file.write_all(b"\n").await?;
        file.flush().await?;

        Ok(())
    }

    async fn get_target_file(&self) -> Result<PathBuf, std::io::Error> {
        let mut current = self.current_file.lock().await;

        match self.rotation_strategy {
            RotationStrategy::Daily => {
                let today = Utc::now().format("%Y-%m-%d").to_string();
                let candidate = self.base_path.with_file_name(format!(
                    "{}-{}.log",
                    self.base_path.file_stem().unwrap_or_default().to_string_lossy(),
                    today
                ));

                if current.as_ref() != Some(&candidate) {
                    if let Some(parent) = candidate.parent() {
                        fs::create_dir_all(parent).await?;
                    }
                    *current = Some(candidate.clone());
                }

                Ok(candidate)
            }
            RotationStrategy::Size => {
                let candidate = self.base_path.clone();

                if current.as_ref() != Some(&candidate) {
                    if let Some(parent) = candidate.parent() {
                        fs::create_dir_all(parent).await?;
                    }
                    *current = Some(candidate.clone());
                }

                if candidate.exists() {
                    let metadata = fs::metadata(&candidate).await?;
                    if metadata.len() >= self.max_size_bytes {
                        let timestamp = Utc::now().format("%Y%m%d-%H%M%S").to_string();
                        let rotated = candidate.with_file_name(format!(
                            "{}-{}.log",
                            candidate.file_stem().unwrap_or_default().to_string_lossy(),
                            timestamp
                        ));
                        fs::rename(&candidate, &rotated).await?;
                    }
                }

                Ok(candidate)
            }
        }
    }
}
