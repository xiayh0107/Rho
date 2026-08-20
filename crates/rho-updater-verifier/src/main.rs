use anyhow::{Context, Result, bail, ensure};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use minisign_verify::{PublicKey, Signature};
use serde_json::Value;
use std::{
    env,
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

const MAX_CONFIG_BYTES: u64 = 256 * 1024;
const MAX_SIGNATURE_BYTES: u64 = 16 * 1024;
const MAX_ARTIFACT_BYTES: u64 = 1024 * 1024 * 1024;

struct Arguments {
    config: PathBuf,
    artifact: PathBuf,
    signature: PathBuf,
}

fn parse_arguments() -> Result<Arguments> {
    let mut config = None;
    let mut artifact = None;
    let mut signature = None;
    let mut args = env::args_os().skip(1);
    while let Some(flag) = args.next() {
        let value = args.next().context("each flag requires a value")?;
        match flag.to_str() {
            Some("--config") if config.is_none() => config = Some(PathBuf::from(value)),
            Some("--artifact") if artifact.is_none() => artifact = Some(PathBuf::from(value)),
            Some("--signature") if signature.is_none() => signature = Some(PathBuf::from(value)),
            _ => {
                bail!("usage: rho-updater-verifier --config FILE --artifact FILE --signature FILE")
            }
        }
    }
    Ok(Arguments {
        config: config.context("--config is required")?,
        artifact: artifact.context("--artifact is required")?,
        signature: signature.context("--signature is required")?,
    })
}

fn checked_file(path: &Path, label: &str, maximum: u64) -> Result<u64> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("{label} is unavailable: {}", path.display()))?;
    ensure!(
        !metadata.file_type().is_symlink() && metadata.is_file(),
        "{label} must be a regular file"
    );
    ensure!(
        metadata.len() > 0 && metadata.len() <= maximum,
        "{label} is empty or exceeds its byte budget"
    );
    Ok(metadata.len())
}

fn read_bounded(path: &Path, label: &str, maximum: u64) -> Result<Vec<u8>> {
    let expected_length = checked_file(path, label, maximum)?;
    let bytes = fs::read(path).with_context(|| format!("could not read {label}"))?;
    ensure!(
        bytes.len() as u64 == expected_length && bytes.len() as u64 <= maximum,
        "{label} changed while being read"
    );
    Ok(bytes)
}

fn next_artifact_read_length(current: u64, incoming: usize, expected: u64) -> Result<u64> {
    let next = current
        .checked_add(incoming as u64)
        .context("Tauri updater artifact read length overflow")?;
    ensure!(
        next <= MAX_ARTIFACT_BYTES && next <= expected,
        "Tauri updater artifact changed or exceeds its byte budget"
    );
    Ok(next)
}

fn decode_canonical_base64(value: &str, label: &str) -> Result<Vec<u8>> {
    ensure!(!value.is_empty(), "{label} is empty");
    let decoded = STANDARD
        .decode(value)
        .with_context(|| format!("{label} is not base64"))?;
    ensure!(
        STANDARD.encode(&decoded) == value,
        "{label} is not canonical base64"
    );
    Ok(decoded)
}

fn exact_lines(value: &str, count: usize, label: &str) -> Result<()> {
    ensure!(
        value.lines().count() == count
            && value
                .chars()
                .all(|character| !character.is_control() || matches!(character, '\n' | '\t')),
        "{label} has an invalid textual shape"
    );
    Ok(())
}

fn updater_public_key(config_path: &Path) -> Result<PublicKey> {
    let config: Value = serde_json::from_slice(&read_bounded(
        config_path,
        "Tauri configuration",
        MAX_CONFIG_BYTES,
    )?)
    .context("Tauri configuration is not JSON")?;
    let encoded = config
        .pointer("/plugins/updater/pubkey")
        .and_then(Value::as_str)
        .context("Tauri updater public key is missing")?;
    let decoded = decode_canonical_base64(encoded, "Tauri updater public key")?;
    let decoded = String::from_utf8(decoded).context("Tauri updater public key is not UTF-8")?;
    exact_lines(&decoded, 2, "Tauri updater public key")?;
    PublicKey::decode(&decoded).context("Tauri updater public key is invalid")
}

fn updater_signature(signature_path: &Path) -> Result<Signature> {
    let bytes = read_bounded(
        signature_path,
        "Tauri updater signature",
        MAX_SIGNATURE_BYTES,
    )?;
    let encoded = std::str::from_utf8(&bytes)
        .context("Tauri updater signature is not UTF-8")?
        .trim();
    let decoded = decode_canonical_base64(encoded, "Tauri updater signature")?;
    let decoded = String::from_utf8(decoded).context("Tauri updater signature is not UTF-8")?;
    exact_lines(&decoded, 4, "Tauri updater signature")?;
    Signature::decode(&decoded).context("Tauri updater signature is invalid")
}

fn verify(config_path: &Path, artifact_path: &Path, signature_path: &Path) -> Result<()> {
    let public_key = updater_public_key(config_path)?;
    let signature = updater_signature(signature_path)?;
    let expected_length =
        checked_file(artifact_path, "Tauri updater artifact", MAX_ARTIFACT_BYTES)?;
    let mut file = File::open(artifact_path).context("could not open Tauri updater artifact")?;
    let mut verifier = public_key
        .verify_stream(&signature)
        .context("Tauri updater signature must use the current prehashed Minisign format")?;
    let mut buffer = [0_u8; 64 * 1024];
    let mut read_length = 0_u64;
    loop {
        let read = file
            .read(&mut buffer)
            .context("could not read Tauri updater artifact")?;
        if read == 0 {
            break;
        }
        read_length = next_artifact_read_length(read_length, read, expected_length)?;
        verifier.update(&buffer[..read]);
    }
    ensure!(
        read_length == expected_length,
        "Tauri updater artifact changed while being verified"
    );
    verifier
        .finalize()
        .context("Tauri updater signature does not verify against the configured public key")
}

fn run() -> Result<()> {
    let args = parse_arguments()?;
    verify(&args.config, &args.artifact, &args.signature)?;
    let artifact_name = args
        .artifact
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("artifact");
    println!("Verified Tauri updater signature for {artifact_name}.");
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("rho-updater-verifier: {error:#}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PUBLIC_KEY: &str = "untrusted comment: minisign public key E7620F1842B4E81F\nRWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3";
    const PREHASHED_SIGNATURE: &str = "untrusted comment: signature from minisign secret key\nRUQf6LRCGA9i559r3g7V1qNyJDApGip8MfqcadIgT9CuhV3EMhHoN1mGTkUidF/z7SrlQgXdy8ofjb7bNJJylDOocrCo8KLzZwo=\ntrusted comment: timestamp:1556193335\tfile:test\ny/rUw2y8/hOUYjZU71eHp/Wo1KZ40fGy2VJEDl34XMJM+TX48Ss/17u3IvIfbVR1FkZZSNCisQbuQY+bHwhEBg==";

    #[test]
    fn accepts_a_tauri_encoded_prehashed_signature() {
        let public_key = PublicKey::decode(PUBLIC_KEY).unwrap();
        exact_lines(PREHASHED_SIGNATURE, 4, "signature").unwrap();
        let encoded_signature = STANDARD.encode(PREHASHED_SIGNATURE);
        let decoded = decode_canonical_base64(&encoded_signature, "signature").unwrap();
        let signature = Signature::decode(std::str::from_utf8(&decoded).unwrap()).unwrap();
        let mut verifier = public_key.verify_stream(&signature).unwrap();
        verifier.update(b"te");
        verifier.update(b"st");
        verifier.finalize().unwrap();
    }

    #[test]
    fn rejects_noncanonical_or_extra_signature_content() {
        assert!(decode_canonical_base64("not base64", "signature").is_err());
        assert!(exact_lines("one\ntwo\nthree\nfour\nfive", 4, "signature").is_err());
    }

    #[test]
    fn bounds_the_streamed_artifact_even_if_its_file_changes() {
        assert_eq!(next_artifact_read_length(0, 4, 4).unwrap(), 4);
        assert!(next_artifact_read_length(4, 1, 4).is_err());
        assert!(next_artifact_read_length(MAX_ARTIFACT_BYTES, 1, MAX_ARTIFACT_BYTES + 1).is_err());
        assert!(next_artifact_read_length(u64::MAX, 1, u64::MAX).is_err());
    }
}
