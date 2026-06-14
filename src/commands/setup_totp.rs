use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use rand::rngs::OsRng;
use rand::RngCore;
use totp_lite;

pub async fn run() -> anyhow::Result<()> {
    let secret_path = ".lunar/totp.secret";

    if std::path::Path::new(secret_path).exists() {
        // Reset flow – require current TOTP
        let secret = fs::read_to_string(secret_path)?.trim().to_string();
        println!("TOTP already configured.");
        print!("Enter current TOTP code to reset (or Ctrl+C to cancel): ");
        io::stdout().flush()?;
        let mut code = String::new();
        io::stdin().read_line(&mut code)?;
        let code = code.trim();
        // Verify
        let expected = totp_lite::totp::<totp_lite::Sha1>(
            secret.as_bytes(),
            chrono::Utc::now().timestamp() as u64,
        );
        if expected != code {
            anyhow::bail!("Invalid TOTP code. Reset aborted.");
        }
        println!("Current TOTP verified.");
    }

    // Generate new secret
    let mut seed = [0u8; 20];
    OsRng.fill_bytes(&mut seed);
    let secret = data_encoding::BASE32_NOPAD.encode(&seed);

    // Write with strict permissions
    if let Some(parent) = std::path::Path::new(secret_path).parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(secret_path, &secret)?;
    fs::set_permissions(secret_path, std::fs::Permissions::from_mode(0o600))?;

    let uri = format!(
        "otpauth://totp/LunarAST?secret={}&issuer=LunarAST&digits=6",
        secret
    );

    println!("\n✅ TOTP secret saved to {}", secret_path);
    println!("Scan the QR code below with your authenticator app:\n");

    // Print ASCII QR code using qrcode crate
    if let Ok(qr_code) = qrcode::QrCode::new(uri.as_bytes()) {
        let qr_str = qr_code
            .render::<qrcode::render::unicode::Dense1x2>()
            .quiet_zone(false)
            .module_dimensions(2, 1)
            .build();
        println!("{}", qr_str);
    } else {
        println!("(QR code generation failed, use manual entry)");
    }

    println!("\nOr enter this secret manually: {}", secret);
    println!("\nNext: visit your LunarAST web, click Login and use the TOTP code.");
    Ok(())
}
