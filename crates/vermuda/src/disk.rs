use anyhow::{Context, Result};
use log::info;
use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

fn format_size(size_mb: u64) -> String {
    if size_mb >= 1024 {
        format!("{:.1}GB", size_mb as f64 / 1024.0)
    } else {
        format!("{}MB", size_mb)
    }
}

pub struct DiskImage {
    path: PathBuf,
    size: u64,
}

impl DiskImage {
    pub fn new<P: AsRef<Path>>(path: P, size: u64) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            size,
        }
    }

    pub fn ensure_exists(&self) -> Result<()> {
        if self.path.exists() {
            info!(
                "Using existing disk image: {} ({})",
                self.path.display(),
                format_size(self.size)
            );
            return Ok(());
        }

        info!(
            "Creating disk image: {} ({})",
            self.path.display(),
            format_size(self.size)
        );
        self.create_raw_image()?;
        info!("Disk image created successfully");
        Ok(())
    }

    fn create_raw_image(&self) -> Result<()> {
        let size_bytes = self.size * 1024 * 1024;

        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.path)
            .with_context(|| format!("Failed to create disk image at {}", self.path.display()))?;

        file.seek(SeekFrom::Start(size_bytes - 1))
            .context("Failed to seek in disk image")?;

        file.write_all(&[0])
            .context("Failed to write to disk image")?;

        file.sync_all()
            .context("Failed to sync disk image to storage")?;

        Ok(())
    }
}
