use anyhow::{anyhow, Result};
use std::path::Path;
use s3::creds::Credentials;
use s3::{Bucket, Region};

/// Upload a file to an S3-compatible storage bucket (Cloudflare R2, AWS S3, MinIO, etc.).
///
/// Credentials are read from environment variables (zero hardcode):
///   - `AWS_ACCESS_KEY_ID`
///   - `AWS_SECRET_ACCESS_KEY`
///   - `AWS_ENDPOINT_URL`
///
/// Size limit per file: `LUNAR_MAX_UPLOAD_SIZE_MB` (default 10 MB)
/// Total bucket capacity: `LUNAR_MAX_BUCKET_SIZE_GB` (default 10 GB)
/// Stop threshold: `LUNAR_BUCKET_STOP_THRESHOLD` (default 0.98, i.e. 98%)
pub async fn upload_to_s3(
    local_file_path: &Path,
    target_key: &str,
    bucket_name: &str,
) -> Result<()> {
    let access_key = std::env::var("AWS_ACCESS_KEY_ID")
        .map_err(|_| anyhow!("Missing AWS_ACCESS_KEY_ID environment variable"))?;
    let secret_key = std::env::var("AWS_SECRET_ACCESS_KEY")
        .map_err(|_| anyhow!("Missing AWS_SECRET_ACCESS_KEY environment variable"))?;
    let endpoint = std::env::var("AWS_ENDPOINT_URL")
        .map_err(|_| anyhow!("Missing AWS_ENDPOINT_URL environment variable"))?;

    // Check single file size
    let metadata = std::fs::metadata(local_file_path)?;
    let file_size_bytes = metadata.len();
    let file_size_mb = file_size_bytes as f64 / (1024.0 * 1024.0);
    let max_size_mb: f64 = std::env::var("LUNAR_MAX_UPLOAD_SIZE_MB")
        .unwrap_or_else(|_| "10".to_string())
        .parse()
        .unwrap_or(10.0);

    if file_size_mb > max_size_mb {
        anyhow::bail!(
            "File size ({:.1} MB) exceeds upload limit ({:.0} MB). Aborting.",
            file_size_mb,
            max_size_mb
        );
    }

    let credentials = Credentials::new(
        Some(&access_key),
        Some(&secret_key),
        None,
        None,
        None,
    )?;

    let region = Region::Custom {
        region: "auto".to_string(),
        endpoint,
    };

    let bucket = Bucket::new(bucket_name, region, credentials)?;

    // Check total bucket capacity (if configured)
    if let Ok(max_gb_str) = std::env::var("LUNAR_MAX_BUCKET_SIZE_GB") {
        let max_gb: f64 = max_gb_str.parse().unwrap_or(10.0);
        let threshold: f64 = std::env::var("LUNAR_BUCKET_STOP_THRESHOLD")
            .unwrap_or_else(|_| "0.98".to_string())
            .parse()
            .unwrap_or(0.98);
        let max_bytes = (max_gb * 1024.0 * 1024.0 * 1024.0) as u64;
        let stop_bytes = (max_bytes as f64 * threshold) as u64;

        let mut current_usage: u64 = 0;
        let mut continuation_token: Option<String> = None;
        loop {
            let (result, _code) = bucket
                .list_page(
                    "".to_string(),
                    None,
                    continuation_token,
                    None,
                    None, // max_keys
                )
                .await?;
            for obj in result.contents.iter() {
                current_usage += obj.size;
            }
            continuation_token = result.continuation_token;
            if continuation_token.is_none() {
                break;
            }
        }

        if current_usage + file_size_bytes > stop_bytes {
            anyhow::bail!(
                "Bucket usage ({:.1} GB) would exceed stop threshold ({:.0}% of {:.0} GB). Aborting.",
                current_usage as f64 / (1024.0 * 1024.0 * 1024.0),
                threshold * 100.0,
                max_gb
            );
        }
    }

    let content = std::fs::read(local_file_path)
        .map_err(|e| anyhow!("Failed to read local file: {}", e))?;

    let response = bucket
        .put_object_with_content_type(target_key, &content, "application/json")
        .await?;

    if response.status_code() != 200 {
        anyhow::bail!(
            "Upload failed. HTTP {}: {}",
            response.status_code(),
            String::from_utf8_lossy(response.as_slice())
        );
    }

    println!("✓ Uploaded to {}/{}", bucket_name, target_key);
    Ok(())
}
