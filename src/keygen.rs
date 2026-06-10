use anyhow::Result;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use std::fs;
use std::os::unix::fs::OpenOptionsExt;

/// Generate an Ed25519 key pair for project signing.
/// Private key is written to ~/.lunar/keys/<project>.key with POSIX 0600 permissions.
/// Public key is returned (register with lunar-gateway as LUNAR_PUBLIC_KEYS).
pub fn generate_keypair(project_name: &str) -> Result<()> {
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let verifying_key = signing_key.verifying_key();

    let mut key_dir = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Failed to locate home directory"))?;
    key_dir.push(".lunar/keys");
    fs::create_dir_all(&key_dir)?;

    let mut key_path = key_dir.clone();
    key_path.push(format!("{}.key", project_name));

    let private_hex = hex::encode(signing_key.to_bytes());
    let public_hex = hex::encode(verifying_key.as_bytes());

    #[cfg(unix)]
    {
        use std::io::Write;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&key_path)?;
        file.write_all(private_hex.as_bytes())?;
    }
    #[cfg(not(unix))]
    {
        fs::write(&key_path, &private_hex)?;
    }

    println!("🔑 Key pair generated for project: {}", project_name);
    println!();
    println!("  Register this public key in lunar-gateway (LUNAR_PUBLIC_KEYS):");
    println!("  \"{}\": \"{}\"", project_name, public_hex);
    println!();
    println!("  🔒 Private key written to {}", key_path.display());
    println!("     Permissions: 600 (owner read/write only)");

    Ok(())
}
