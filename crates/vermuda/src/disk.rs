use anyhow::{Context, Result};
use log::info;
use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

pub struct DiskImage {
    path: PathBuf,
    size_gb: f64,
}

impl DiskImage {
    pub fn new<P: AsRef<Path>>(path: P, size_gb: f64) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            size_gb,
        }
    }

    pub fn ensure_exists(&self) -> Result<()> {
        if self.path.exists() {
            info!(
                "Using existing disk image: {} ({:.1}GB)",
                self.path.display(),
                self.size_gb
            );
            return Ok(());
        }

        info!(
            "Creating disk image: {} ({:.1}GB)",
            self.path.display(),
            self.size_gb
        );
        self.create_raw_image()?;
        info!("Disk image created successfully");
        Ok(())
    }

    fn create_raw_image(&self) -> Result<()> {
        let size_bytes = (self.size_gb * 1024.0 * 1024.0 * 1024.0) as u64;

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

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn size_gb(&self) -> f64 {
        self.size_gb
    }
}
