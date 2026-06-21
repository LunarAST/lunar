use anyhow::Result;
use std::io::{self, Write};
use crate::adapter::find_adapter;

pub fn execute() -> Result<()> {
    let default_port = std::env::var("LUNAR_SERVE_PORT").unwrap_or_else(|_| "8787".to_string());
    
    print!("Starting lunar-serve (Default port: {}). Do you want to continue? [Y/n/custom-port]: ", default_port);
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let trimmed = input.trim();
    
    let port_str = if trimmed.is_empty() || trimmed.to_lowercase() == "y" {
        default_port
    } else if trimmed.to_lowercase() == "n" {
        println!("Serve launch cancelled.");
        return Ok(());
    } else {
        trimmed.to_string()
    };
    
    println!("🚀 Spawning lunar-serve on port {}...", port_str);
    
    let binary_name = "lunar-serve";
    let binary_path = find_adapter(binary_name)
        .ok_or_else(|| anyhow::anyhow!("Binary 'lunar-serve' not found in PATH. Ensure it is compiled and installed."))?;
        
    let map_path = "lunar-map.json";
    // 读取当前域设置，传递给子进程
    let domain = std::env::var("LUNAR_SERVE_DOMAIN").unwrap_or_else(|_| String::new());
    
    let mut child = std::process::Command::new(binary_path)
        .arg(map_path)
        .env("LUNAR_SERVE_PORT", &port_str)
        .env("LUNAR_SERVE_DOMAIN", &domain)
        .spawn()?;
        
    let status = child.wait()?;
    if !status.success() {
        anyhow::bail!("lunar-serve exited with an error status.");
    }
    Ok(())
}
