use lunar::uploader;
use std::path::Path;

#[tokio::test]
async fn test_uploader_errors() {
    // Sub-test 1: missing credentials
    std::env::remove_var("AWS_ACCESS_KEY_ID");
    std::env::remove_var("AWS_SECRET_ACCESS_KEY");
    std::env::remove_var("AWS_ENDPOINT_URL");
    let result = uploader::upload_to_s3(
        Path::new("/tmp/nonexistent-file"),
        "test-key",
        "test-bucket",
    )
    .await;
    assert!(result.is_err());

    // Sub-test 2: file too large
    std::env::set_var("LUNAR_MAX_UPLOAD_SIZE_MB", "0.001");
    std::env::set_var("AWS_ACCESS_KEY_ID", "test-key-id");
    std::env::set_var("AWS_SECRET_ACCESS_KEY", "test-secret");
    std::env::set_var("AWS_ENDPOINT_URL", "https://example.com");

    let large_file = "/tmp/lunar-upload-test-large.bin";
    std::fs::write(large_file, vec![0u8; 2048]).unwrap();

    let result = uploader::upload_to_s3(
        Path::new(large_file),
        "test-key",
        "test-bucket",
    )
    .await;
    assert!(result.is_err(), "Expected an error due to file size limit");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("exceeds upload limit") || err_msg.contains("upload limit"),
        "Error should mention upload limit, but got: {}",
        err_msg
    );

    std::fs::remove_file(large_file).ok();
    std::env::remove_var("LUNAR_MAX_UPLOAD_SIZE_MB");
}
