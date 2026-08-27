use std::{
    env,
    error::Error,
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

fn main() -> Result<(), Box<dyn Error>> {
    let module_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    generate_asset_module(
        &module_dir.join("assets/characters"),
        "png",
        "character_thumbnails.rs",
        "character_thumbnail",
    )?;
    generate_asset_module(
        &module_dir.join("assets/regions"),
        "webp",
        "region_icons.rs",
        "region_icon",
    )?;
    generate_asset_module(
        &module_dir.join("assets/weapons-hoyowiki"),
        "png",
        "weapon_hoyowiki_thumbnails.rs",
        "weapon_hoyowiki_thumbnail",
    )?;
    generate_asset_module(
        &module_dir.join("assets/weapons"),
        "png",
        "weapon_fallback_thumbnails.rs",
        "weapon_fallback_thumbnail",
    )?;
    generate_asset_module(
        &module_dir.join("assets/skins"),
        "png",
        "skin_png_thumbnails.rs",
        "skin_png_thumbnail",
    )?;
    generate_asset_module(
        &module_dir.join("assets/skins"),
        "gif",
        "skin_gif_thumbnails.rs",
        "skin_gif_thumbnail",
    )?;
    generate_asset_module(
        &module_dir.join("assets/artifacts"),
        "png",
        "artifact_thumbnails.rs",
        "artifact_thumbnail",
    )
}

fn generate_asset_module(
    asset_dir: &Path,
    extension: &str,
    output_file: &str,
    function_name: &str,
) -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed={}", asset_dir.display());

    let mut assets = Vec::new();
    for entry in fs::read_dir(asset_dir)? {
        let path = entry?.path();
        if path.extension().and_then(|value| value.to_str()) != Some(extension) {
            continue;
        }
        let source_key = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or("asset filename must be UTF-8")?;
        if !source_key.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-' || byte == b'_'
        }) {
            return Err(format!("invalid asset source key: {source_key}").into());
        }
        assets.push((source_key.to_string(), path.canonicalize()?));
    }
    assets.sort_by(|left, right| left.0.cmp(&right.0));

    let mut generated = format!(
        "pub fn {function_name}(source_key: &str) -> Option<&'static [u8]> {{\n    match source_key {{\n"
    );
    for (source_key, path) in &assets {
        writeln!(
            generated,
            "        \"{source_key}\" => Some(include_bytes!(r#\"{}\"#)),",
            path.display()
        )?;
    }
    generated.push_str("        _ => None,\n    }\n}\n");

    fs::write(
        PathBuf::from(env::var("OUT_DIR")?).join(output_file),
        generated,
    )?;
    Ok(())
}
