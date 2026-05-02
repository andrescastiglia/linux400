use clap::Parser;
use l400::{bootstrap_l400_root, resolve_l400_root};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "l400-bootstrap",
    about = "Provisiona bibliotecas y objetos base de Linux/400"
)]
struct Args {
    #[arg(long, value_name = "PATH")]
    root: Option<PathBuf>,

    #[arg(long)]
    quiet: bool,
}

fn main() -> ExitCode {
    let args = Args::parse();
    let root = args.root.unwrap_or_else(resolve_l400_root);

    match bootstrap_l400_root(&root) {
        Ok(report) => {
            if !args.quiet {
                println!("Linux/400 bootstrap root={}", report.root.display());
                println!("created={}", report.created.len());
                println!("existing={}", report.existing.len());
                for item in report.created {
                    println!("created {}", item);
                }
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("l400-bootstrap: {error}");
            ExitCode::FAILURE
        }
    }
}
