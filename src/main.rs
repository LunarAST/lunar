use clap::{Parser, Subcommand};
use lunar::commands::{scan, diff, sync, pull, serve, interactive};
use lunar::{
    doctor::doctor_check,
    cleanup::{cleanup_local, cleanup_archives},
};
use std::path::Path;
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "lunar")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Scan,
    Diff,
    Sync {
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        dry_run: bool,
    },
    Pull {
        #[arg(short, long)]
        project: Option<String>,
        #[arg(short, long)]
        yes: bool,
    },
    Serve,
    Map {
        #[arg(short = 'c', long)]
        config: Option<String>,
        #[arg(short = 'o', long)]
        output: Option<String>,
        #[arg(long)]
        upload: bool,
        #[arg(long, requires = "upload")]
        bucket: Option<String>,
        #[arg(long)]
        yes: bool,
    },
    Doctor,
    Cleanup {
        #[arg(long)]
        all: bool,
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        archive: bool,
        #[arg(long, default_value_t = 30)]
        days: i64,
    },
    Patch {
        file: Option<String>,
    },
    Keygen {
        #[arg(default_value_t = current_dir_project_name())]
        project: String,
    },
    Share,
}

fn current_dir_project_name() -> String {
    std::env::current_dir()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "unknown".to_string())
}

#[tokio::main]
async fn main() -> ExitCode {
    if std::env::args().len() == 1 {
        return interactive::run().await;
    }

    let cli = Cli::parse();
    let command = match cli.command {
        Some(cmd) => cmd,
        None => return interactive::run().await,
    };

    let result = match command {
        Commands::Scan => scan::execute(),
        Commands::Diff => diff::execute(),
        Commands::Sync { apply, dry_run } => sync::execute(apply, dry_run),
        Commands::Pull { project, yes } => pull::execute(project, yes).await,
        Commands::Serve => serve::execute(),
        Commands::Map { config, output, upload, bucket, yes } => {
            lunar::map::map(config.as_deref(), output.as_deref(), upload, bucket.as_deref(), yes).await
        }
        Commands::Doctor => { return doctor_check(); }
        Commands::Cleanup { all: _, yes, archive, days } => {
            if archive {
                cleanup_archives(Path::new("."), days, yes)
            } else {
                cleanup_local(yes).map(|_| ())
            }
        }
        Commands::Patch { file } => lunar::patch::patch_cmd(file),
        Commands::Keygen { project } => {
            match lunar::keygen::generate_keypair(&project) {
                Ok(()) => Ok(()),
                Err(e) => { eprintln!("Error: {}", e); Ok(()) }
            }
        }
        Commands::Share => {
            match lunar::share::run_share().await {
                Ok(()) => Ok(()),
                Err(e) => { eprintln!("Error: {}", e); Ok(()) }
            }
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        ExitCode::from(1)
    } else {
        ExitCode::from(0)
    }
}
