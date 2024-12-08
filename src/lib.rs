use std::{
    borrow::Cow,
    env::current_dir,
    fs::{self, File},
    path::Path,
    sync::LazyLock,
};

use anyhow::{anyhow, bail, Result};
use flate2::{write::GzEncoder, Compression};
use regex::Regex;
use sha2::{Digest, Sha256 as Sha256Hasher};
use tracing::info;

pub mod commands;
pub mod package;

use package::{Info, Package};

static VARIABLE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"%\{([a-zA-Z0-9_]*)\}").expect("invalid regex"));

pub fn check_hash<P: AsRef<Path>>(path: P, hash: &str) -> Result<bool> {
    let file = fs::read(path)?;
    let (hash_type, hash) = hash
        .split_once(':')
        .ok_or(anyhow!("Invalid checksum format"))?;

    let computed_hash = match hash_type {
        "blake3" => blake3::hash(&file).to_hex().to_string(),
        "sha256" => base16ct::lower::encode_string(Sha256Hasher::digest(&file).as_slice()),
        _ => bail!("Unsupported hash"),
    };

    Ok(hash == computed_hash)
}

pub fn create_tarball<P: AsRef<Path>>(package_path: P, package: &Package) -> Result<()> {
    let tarball_name = format!("{}-{}.peach", package.info.name, package.info.version);
    let tarball_path = current_dir()?.join(&tarball_name);
    let tar_gz = File::create(&tarball_path)?;
    let enc = GzEncoder::new(tar_gz, Compression::default());
    let mut tar = tar::Builder::new(enc);

    tar.append_dir_all(".", package_path)?;

    info!("Created package: {}", tarball_name);
    Ok(())
}
