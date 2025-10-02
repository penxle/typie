use cargo_metadata::MetadataCommand;
use fatfs::{FormatVolumeOptions, FsOptions, format_volume};
use fscommon::BufStream;
use std::{env, fs, io::Write, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let metadata = MetadataCommand::new().exec()?;

    let efi_path = metadata
        .target_directory
        .join("aarch64-unknown-uefi")
        .join("release")
        .join("vermuda-boot.efi")
        .into_std_path_buf();

    if !efi_path.exists() {
        eprintln!("Error: EFI binary not found at {}", efi_path.display());
        std::process::exit(1);
    }

    let efi_data = fs::read(&efi_path)?;

    let img_path = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?).join("vermuda-boot.img");
    let img = fs::File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&img_path)?;
    img.set_len(10 * 1024 * 1024)?;

    let mut buffer = BufStream::new(img);

    format_volume(
        &mut buffer,
        FormatVolumeOptions::new().volume_label(*b"VERMUDABOOT"),
    )?;

    let fs = fatfs::FileSystem::new(buffer, FsOptions::new())?;
    fs.root_dir()
        .create_dir("EFI")?
        .create_dir("BOOT")?
        .create_file("BOOTAA64.EFI")?
        .write_all(&efi_data)?;

    println!("✓ Packed EFI bootable image: {}", img_path.display());

    Ok(())
}
