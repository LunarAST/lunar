use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use rand::rngs::OsRng;
use rand::RngCore;
use totp_lite;

/// Verify a TOTP code against a Base32-encoded secret (with ±30s window)
fn verify_current_totp(code: &str, secret_b32: &str) -> bool {
    let secret_b32_upper = secret_b32.trim().to_uppercase();
    let secret_bytes = match data_encoding::BASE32_NOPAD.decode(secret_b32_upper.as_bytes()) {
        Ok(b) => b,
        Err(_) => return false,
    };
    let now = chrono::Utc::now().timestamp() as u64;
    for offset in [-1i64, 0, 1].iter() {
        let t = (now as i64 + offset * 30) as u64;
        // Force 6-digit output
        let expected = totp_lite::totp_custom::<totp_lite::Sha1>(30, 6, &secret_bytes, t);
        if expected == code {
            return true;
        }
    }
    false
}

pub async fn run() -> anyhow::Result<()> {
    let secret_path = ".lunar/totp.secret";

    if std::path::Path::new(secret_path).exists() {
        let secret = fs::read_to_string(secret_path)?.trim().to_string();
        println!("TOTP already configured.");
        print!("Enter current TOTP code to reset (or Ctrl+C to cancel): ");
        io::stdout().flush()?;
        let mut code = String::new();
        io::stdin().read_line(&mut code)?;
        let code = code.trim();
        if !verify_current_totp(code, &secret) {
            println!("Invalid TOTP code. Reset aborted.");
            println!("If you lost access to your old TOTP, you can emergency reset by deleting the secret file:");
            println!("  rm {}", secret_path);
            println!("Then run this command again.");
            anyhow::bail!("Invalid TOTP code.");
        }
        println!("Current TOTP verified.");
    }

    let mut seed = [0u8; 20];
    OsRng.fill_bytes(&mut seed);
    let secret = data_encoding::BASE32_NOPAD.encode(&seed);

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
