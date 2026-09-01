use std::{env, error::Error, fmt::Write as _, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let module_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let asset_dir = module_dir.join("assets/characters");
    println!("cargo:rerun-if-changed={}", asset_dir.display());

    let mut assets = Vec::new();
    for entry in fs::read_dir(&asset_dir)? {
        let path = entry?.path();
        if path.extension().and_then(|value| value.to_str()) != Some("png") {
            continue;
        }
        let source_key = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or("character asset filename must be UTF-8")?;
        if !source_key.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-' || byte == b'_'
        }) {
            return Err(format!("invalid character asset source key: {source_key}").into());
        }
        assets.push((source_key.to_string(), path.canonicalize()?));
    }
    assets.sort_by(|left, right| left.0.cmp(&right.0));

    let mut generated = String::from(
        "pub fn character_thumbnail_asset(source_key: &str) -> Option<&'static [u8]> {\n    match source_key {\n",
    );
    for (source_key, path) in &assets {
        writeln!(
            generated,
            "        \"{source_key}\" => Some(include_bytes!(r#\"{}\"#)),",
            path.display()
        )?;
    }
    generated.push_str("        _ => None,\n    }\n}\n");

    let output_path = PathBuf::from(env::var("OUT_DIR")?).join("character_thumbnails.rs");
    fs::write(output_path, generated)?;
    Ok(())
}
