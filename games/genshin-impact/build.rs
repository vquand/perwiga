use std::{
    env,
    error::Error,
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

fn main() -> Result<(), Box<dyn Error>> {
    let module_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    generate_thumbnail_module(&module_dir.join("assets/characters"))
}

fn generate_thumbnail_module(asset_dir: &Path) -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed={}", asset_dir.display());

    let mut thumbnails = Vec::new();
    for entry in fs::read_dir(asset_dir)? {
        let path = entry?.path();
        if path.extension().and_then(|value| value.to_str()) != Some("png") {
            continue;
        }
        let source_key = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or("thumbnail filename must be UTF-8")?;
        if !source_key.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-' || byte == b'_'
        }) {
            return Err(format!("invalid thumbnail source key: {source_key}").into());
        }
        thumbnails.push((source_key.to_string(), path.canonicalize()?));
    }
    thumbnails.sort_by(|left, right| left.0.cmp(&right.0));

    let mut generated = String::from(
        "pub fn character_thumbnail(source_key: &str) -> Option<&'static [u8]> {\n    match source_key {\n",
    );
    for (source_key, path) in &thumbnails {
        writeln!(
            generated,
            "        \"{source_key}\" => Some(include_bytes!(r#\"{}\"#)),",
            path.display()
        )?;
    }
    generated.push_str("        _ => None,\n    }\n}\n");

    fs::write(
        PathBuf::from(env::var("OUT_DIR")?).join("character_thumbnails.rs"),
        generated,
    )?;
    Ok(())
}
