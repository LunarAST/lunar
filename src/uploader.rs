use anyhow::{anyhow, Result};
use std::path::Path;

/// Upload a file to an S3-compatible storage bucket (Cloudflare R2, AWS S3, MinIO, etc.).
///
/// Credentials are read from environment variables (zero hardcode):
///   - `AWS_ACCESS_KEY_ID`
///   - `AWS_SECRET_ACCESS_KEY`
///   - `AWS_ENDPOINT_URL`  (e.g. https://<account-id>.r2.cloudflarestorage.com)
///
/// Size limit is controlled by `LUNAR_MAX_UPLOAD_SIZE_MB` (default 10 MB).
pub async fn upload_to_s3(
    local_file_path: &Path,
    target_key: &str,
    bucket_name: &str,
) -> Result<()> {
    // Read credentials from environment
    let access_key = std::env::var("AWS_ACCESS_KEY_ID")
        .map_err(|_| anyhow!("Missing AWS_ACCESS_KEY_ID environment variable"))?;
    let secret_key = std::env::var("AWS_SECRET_ACCESS_KEY")
        .map_err(|_| anyhow!("Missing AWS_SECRET_ACCESS_KEY environment variable"))?;
    let endpoint = std::env::var("AWS_ENDPOINT_URL")
        .map_err(|_| anyhow!("Missing AWS_ENDPOINT_URL environment variable"))?;

    // Check file size
    let metadata = std::fs::metadata(local_file_path)?;
    let file_size_mb = metadata.len() as f64 / (1024.0 * 1024.0);
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

    let credentials = s3::creds::Credentials::new(
        Some(&access_key),
        Some(&secret_key),
        None,
        None,
        None,
    )?;

    let region = s3::Region::Custom {
        region: "auto".to_string(),
        endpoint,
    };

    let bucket = s3::Bucket::new(bucket_name, region, credentials)?;

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
