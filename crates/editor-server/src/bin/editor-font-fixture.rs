use std::error::Error;
use std::path::PathBuf;
use std::{env, fs};

use editor_server::font::{build_font, get_font_codepoints};

const CHUNK_SIZE: usize = 200;
const MAX_CHUNKS: usize = 255;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args_os().skip(1);
    let (Some(font), Some(output), None) = (args.next(), args.next(), args.next()) else {
        return Err("usage: editor-font-fixture <font.ttf> <output-dir>".into());
    };
    let ttf = fs::read(font)?;
    let codepoints = get_font_codepoints(&ttf)?;
    let size = CHUNK_SIZE.max(codepoints.len().div_ceil(MAX_CHUNKS));
    let chunks: Vec<Vec<u32>> = codepoints.chunks(size).map(<[u32]>::to_vec).collect();
    let built = build_font(&ttf, &chunks)?;
    let root = PathBuf::from(output).join(&built.hash);
    fs::create_dir_all(root.join("chunks"))?;
    fs::write(root.join("base"), &built.base)?;
    fs::write(root.join("manifest.v1"), &built.manifest)?;
    fs::write(root.join("manifest.v2"), &built.manifest_v2)?;
    for (id, chunk) in built.chunks.iter().enumerate() {
        fs::write(root.join("chunks").join(id.to_string()), chunk)?;
    }
    Ok(())
}
