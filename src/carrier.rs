use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

const MAGIC: &[u8] = b"SIGHFAR_CARRIER_V1";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CarrierMetadata {
    payload_name: String,
    payload_len: u64,
}

#[derive(Debug, Clone)]
pub struct CarrierEmbedResult {
    pub output_path: PathBuf,
    pub payload_name: String,
    pub carrier_size: usize,
    pub payload_size: usize,
}

#[derive(Debug, Clone)]
pub struct CarrierExtractResult {
    pub extracted_path: PathBuf,
    pub payload_name: String,
    pub payload_size: usize,
}

pub fn embed_file(
    carrier_path: &Path,
    payload_path: &Path,
    output_path: &Path,
) -> Result<CarrierEmbedResult> {
    let carrier = fs::read(carrier_path).with_context(|| format!("failed to read carrier file: {}", carrier_path.display()))?;
    let payload = fs::read(payload_path).with_context(|| format!("failed to read payload file: {}", payload_path.display()))?;

    let payload_name = payload_path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| anyhow::anyhow!("payload file name is invalid"))?
        .to_string();
    let metadata = CarrierMetadata {
        payload_name: payload_name.clone(),
        payload_len: payload.len() as u64,
    };
    let metadata_bytes = serde_json::to_vec(&metadata).context("failed to encode carrier metadata")?;

    let mut output = Vec::with_capacity(
        carrier.len() + payload.len() + metadata_bytes.len() + 8 + MAGIC.len(),
    );
    output.extend_from_slice(&carrier);
    output.extend_from_slice(&payload);
    output.extend_from_slice(&metadata_bytes);
    output.extend_from_slice(&(metadata_bytes.len() as u64).to_le_bytes());
    output.extend_from_slice(MAGIC);

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("failed to create output folder: {}", parent.display()))?;
    }
    fs::write(output_path, output)
        .with_context(|| format!("failed to write carrier output: {}", output_path.display()))?;

    Ok(CarrierEmbedResult {
        output_path: output_path.to_path_buf(),
        payload_name,
        carrier_size: carrier.len(),
        payload_size: payload.len(),
    })
}

pub fn extract_file(container_path: &Path, output_dir: &Path) -> Result<CarrierExtractResult> {
    let bytes = fs::read(container_path)
        .with_context(|| format!("failed to read container file: {}", container_path.display()))?;
    if bytes.len() < MAGIC.len() + 8 {
        bail!("container file is too small to contain a hidden payload");
    }
    if !bytes.ends_with(MAGIC) {
        bail!("no SighFar carrier trailer found in this file");
    }

    let magic_start = bytes.len() - MAGIC.len();
    let meta_len_start = magic_start - 8;
    let meta_len = u64::from_le_bytes(
        bytes[meta_len_start..magic_start]
            .try_into()
            .context("carrier metadata length is malformed")?,
    ) as usize;
    if meta_len_start < meta_len {
        bail!("carrier metadata is malformed");
    }
    let meta_start = meta_len_start - meta_len;
    let metadata: CarrierMetadata = serde_json::from_slice(&bytes[meta_start..meta_len_start])
        .context("failed to decode carrier metadata")?;

    let payload_len = metadata.payload_len as usize;
    if meta_start < payload_len {
        bail!("carrier payload length is malformed");
    }
    let payload_start = meta_start - payload_len;
    let payload = &bytes[payload_start..meta_start];

    fs::create_dir_all(output_dir)
        .with_context(|| format!("failed to create extraction folder: {}", output_dir.display()))?;
    let extracted_path = output_dir.join(&metadata.payload_name);
    fs::write(&extracted_path, payload)
        .with_context(|| format!("failed to write extracted payload: {}", extracted_path.display()))?;

    Ok(CarrierExtractResult {
        extracted_path,
        payload_name: metadata.payload_name,
        payload_size: payload_len,
    })
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{embed_file, extract_file};

    fn unique_root(label: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("sighfar-carrier-{label}-{unique}"))
    }

    #[test]
    fn embed_and_extract_round_trip() {
        let root = unique_root("basic");
        std::fs::create_dir_all(&root).unwrap();
        let carrier_path = root.join("carrier.bin");
        let payload_path = root.join("payload.txt");
        let output_path = root.join("wrapped.bin");
        let extract_dir = root.join("out");

        std::fs::write(&carrier_path, b"carrier-bytes").unwrap();
        std::fs::write(&payload_path, b"secret-payload").unwrap();

        let embed = embed_file(&carrier_path, &payload_path, &output_path).unwrap();
        assert_eq!(embed.payload_name, "payload.txt");

        let extract = extract_file(&output_path, &extract_dir).unwrap();
        assert_eq!(extract.payload_name, "payload.txt");
        assert_eq!(
            std::fs::read_to_string(extract.extracted_path).unwrap(),
            "secret-payload"
        );
    }

    #[test]
    fn embed_result_contains_correct_sizes() {
        let root = unique_root("sizes");
        std::fs::create_dir_all(&root).unwrap();
        let carrier_path = root.join("carrier.bin");
        let payload_path = root.join("payload.bin");
        let output_path = root.join("out.bin");

        let carrier_data = b"CARRIER DATA";
        let payload_data = b"PAYLOAD DATA";
        std::fs::write(&carrier_path, carrier_data).unwrap();
        std::fs::write(&payload_path, payload_data).unwrap();

        let result = embed_file(&carrier_path, &payload_path, &output_path).unwrap();
        assert_eq!(result.carrier_size, carrier_data.len());
        assert_eq!(result.payload_size, payload_data.len());
    }

    #[test]
    fn extract_from_non_carrier_file_fails() {
        let root = unique_root("nomagic");
        std::fs::create_dir_all(&root).unwrap();
        let file_path = root.join("plain.bin");
        let extract_dir = root.join("out");

        std::fs::write(&file_path, b"just some random bytes without any magic trailer").unwrap();
        let result = extract_file(&file_path, &extract_dir);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no SighFar carrier trailer"));
    }

    #[test]
    fn extract_from_too_small_file_fails() {
        let root = unique_root("small");
        std::fs::create_dir_all(&root).unwrap();
        let file_path = root.join("tiny.bin");
        let extract_dir = root.join("out");

        std::fs::write(&file_path, b"tiny").unwrap();
        let result = extract_file(&file_path, &extract_dir);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too small"));
    }

    #[test]
    fn embed_and_extract_empty_payload() {
        let root = unique_root("emptypayload");
        std::fs::create_dir_all(&root).unwrap();
        let carrier_path = root.join("carrier.bin");
        let payload_path = root.join("empty.txt");
        let output_path = root.join("out.bin");
        let extract_dir = root.join("extracted");

        std::fs::write(&carrier_path, b"some carrier").unwrap();
        std::fs::write(&payload_path, b"").unwrap();

        let embed = embed_file(&carrier_path, &payload_path, &output_path).unwrap();
        assert_eq!(embed.payload_size, 0);

        let extract = extract_file(&output_path, &extract_dir).unwrap();
        assert_eq!(extract.payload_size, 0);
        assert_eq!(std::fs::read(extract.extracted_path).unwrap(), b"");
    }

    #[test]
    fn embed_and_extract_empty_carrier() {
        let root = unique_root("emptycarrier");
        std::fs::create_dir_all(&root).unwrap();
        let carrier_path = root.join("carrier.bin");
        let payload_path = root.join("payload.txt");
        let output_path = root.join("out.bin");
        let extract_dir = root.join("extracted");

        std::fs::write(&carrier_path, b"").unwrap();
        std::fs::write(&payload_path, b"hidden data").unwrap();

        embed_file(&carrier_path, &payload_path, &output_path).unwrap();
        let extract = extract_file(&output_path, &extract_dir).unwrap();
        assert_eq!(
            std::fs::read_to_string(extract.extracted_path).unwrap(),
            "hidden data"
        );
    }
}
