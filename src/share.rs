use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::process::Command as StdCommand;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::time::sleep;

/// Process guard: automatically kill background processes on exit (Ctrl+C)
struct TunnelGuard {
    serve_process: Option<Child>,
    tunnel_process: Option<Child>,
}

impl Drop for TunnelGuard {
    fn drop(&mut self) {
        println!("\n🧹 Cleaning up resources, releasing port 8787...");
        if let Some(mut child) = self.serve_process.take() {
            let _ = child.start_kill();
        }
        if let Some(mut child) = self.tunnel_process.take() {
            let _ = child.start_kill();
        }
        println!("✓ Resources released.");
    }
}

/// Cross-platform binary locator
fn find_lunar_serve() -> Result<PathBuf> {
    let bin_name = if cfg!(windows) { "lunar-serve.exe" } else { "lunar-serve" };

    let candidates = vec![
        PathBuf::from("/opt/LunarAST/lunar-serve/target/release").join(bin_name),
        PathBuf::from("/opt/LunarAST/lunar-serve/target/debug").join(bin_name),
        PathBuf::from("./target/release").join(bin_name),
        PathBuf::from(bin_name),
    ];
    for c in &candidates {
        if c.exists() {
            return Ok(c.clone());
        }
    }
    // Search PATH (cross-platform)
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join(bin_name);
            if candidate.exists() {
                return Ok(candidate);
            }
        }
    }
    anyhow::bail!("lunar-serve not found. Build it first: cd lunar-serve && cargo build")
}

/// Detect installed tunneling tools (synchronous, using std::process::Command)
fn detect_tunnel_tool() -> Option<&'static str> {
    let tools = vec![("cloudflared", "--version"), ("tailscale", "version"), ("ngrok", "version")];
    for (name, _) in &tools {
        let bin = if cfg!(windows) { format!("{}.exe", name) } else { name.to_string() };
        if StdCommand::new(&bin).output().is_ok() {
            return Some(name);
        }
    }
    if let Ok(tool) = std::env::var("LUNAR_TUNNEL_TOOL") {
        return Some(Box::leak(tool.into_boxed_str()));
    }
    None
}

fn install_hints(tool: &str) -> &str {
    match tool {
        "cloudflared" => "curl -L https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64 -o cloudflared && chmod +x cloudflared",
        "tailscale" => "curl -fsSL https://tailscale.com/install.sh | sh",
        "ngrok" => "Visit https://ngrok.com/download",
        _ => "No known tunneling tool found.",
    }
}

pub async fn run_share() -> Result<()> {
    println!("🌙 LunarAST — Temporary AI Access\n");

    if !Path::new("lunar-map.json").exists() {
        println!("  No lunar-map.json found. Run `lunar map` first to generate topology data.");
        return Ok(());
    }

    let tool = match detect_tunnel_tool() {
        Some(t) => t,
        None => {
            println!("  No tunneling tool found (cloudflared, tailscale, or ngrok).");
            println!();
            println!("  Install one of these tools:");
            println!("    cloudflared: {}", install_hints("cloudflared"));
            println!("    tailscale:   {}", install_hints("tailscale"));
            println!("    ngrok:       {}", install_hints("ngrok"));
            return Ok(());
        }
    };

    println!("  Using tunnel tool: {}", tool);

    let mut guard = TunnelGuard {
        serve_process: None,
        tunnel_process: None,
    };

    // Ensure lunar-serve is running
    let is_running = tokio::net::TcpStream::connect("127.0.0.1:8787").await.is_ok();
    if !is_running {
        println!("  Starting lunar-serve in background...");
        let serve_path = find_lunar_serve()?;
        let child = Command::new(serve_path)
            .arg("lunar-map.json")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()?;
        guard.serve_process = Some(child);
        sleep(Duration::from_millis(800)).await;
    } else {
        println!("  lunar-serve is already running on port 8787");
    }

    // Start tunnel
    println!("  Starting tunnel...");
    let mut tunnel_cmd = Command::new(tool);
    tunnel_cmd
        .args(["tunnel", "--url", "http://localhost:8787"])
        .stderr(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null());

    let mut tunnel_child = tunnel_cmd.spawn()?;
    let stderr = tunnel_child.stderr.take().ok_or_else(|| anyhow!("Failed to capture stderr"))?;
    guard.tunnel_process = Some(tunnel_child);

    // Async URL extraction with 15s timeout
    let mut reader = BufReader::new(stderr).lines();
    let mut public_url = String::new();
    let timeout_fut = sleep(Duration::from_secs(15));
    tokio::pin!(timeout_fut);

    loop {
        tokio::select! {
            line_res = reader.next_line() => {
                if let Ok(Some(line)) = line_res {
                    if let Some(start) = line.find("https://") {
                        if let Some(end) = line[start..].find(".trycloudflare.com") {
                            public_url = line[start..start + end + ".trycloudflare.com".len()].to_string();
                            break;
                        }
                    }
                } else {
                    break;
                }
            }
            _ = &mut timeout_fut => {
                anyhow::bail!("Failed to extract tunnel URL: Timeout after 15 seconds.");
            }
        }
    }

    if public_url.is_empty() {
        anyhow::bail!("Failed to extract URL from cloudflared output. Check your cloudflared version.");
    }

    println!("\n  🌐 Temporary AI access URL:");
    println!("     {}/lunar-map.md?summary=true\n", public_url);
    println!("  Press Ctrl+C to stop the tunnel and release port 8787.");

    tokio::signal::ctrl_c().await?;
    Ok(())
}
