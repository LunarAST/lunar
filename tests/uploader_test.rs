use lunar::uploader;

#[tokio::test]
async fn test_missing_credentials_error() {
    // Ensure no credentials are set
    std::env::remove_var("AWS_ACCESS_KEY_ID");
    std::env::remove_var("AWS_SECRET_ACCESS_KEY");
    std::env::remove_var("AWS_ENDPOINT_URL");

    let result = uploader::upload_to_s3(
        std::path::Path::new("/tmp/nonexistent-file"),
        "test-key",
        "test-bucket",
    )
    .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_file_too_large_rejected() {
    // Set valid credentials but a very small upload limit
    std::env::set_var("AWS_ACCESS_KEY_ID", "test-key-id");
    std::env::set_var("AWS_SECRET_ACCESS_KEY", "test-secret");
    std::env::set_var("AWS_ENDPOINT_URL", "https://example.com");
    std::env::set_var("LUNAR_MAX_UPLOAD_SIZE_MB", "0.001"); // 1 KB limit

    // Create a file larger than 1 KB
    let large_file = "/tmp/lunar-upload-test-large.bin";
    std::fs::write(large_file, vec![0u8; 2048]).unwrap(); // 2 KB

    let result = uploader::upload_to_s3(
        std::path::Path::new(large_file),
        "test-key",
        "test-bucket",
    )
    .await;
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("exceeds upload limit"));

    std::fs::remove_file(large_file).ok();
    std::env::remove_var("LUNAR_MAX_UPLOAD_SIZE_MB");
}
