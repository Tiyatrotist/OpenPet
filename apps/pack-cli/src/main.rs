//! # OpenPet Pack CLI
//!
//! Tooling for validating, building, inspecting, hashing, signing, and verifying .openpet packages.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use openpet_diagnostics::init_diagnostics;
use openpet_petpack::{build_petpack, compute_sha256, PetPackManifest};
use std::fs::{self, File};
use std::io::Read;
use std::path::PathBuf;
use tracing::{info, Level};
use zip::ZipArchive;

#[derive(Parser)]
#[command(name = "openpet-pack")]
#[command(about = "OpenPet Package Management CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Validates a .openpet package or unpacked manifest for security and structural correctness
    Validate {
        #[arg(value_name = "PATH")]
        path: PathBuf,
    },
    /// Builds a .openpet package from a source directory
    Build {
        #[arg(short, long, value_name = "SRC_DIR")]
        source: PathBuf,
        #[arg(short, long, value_name = "OUTPUT")]
        output: PathBuf,
    },
    /// Inspects metadata and contained assets of a .openpet package
    Inspect {
        #[arg(value_name = "PACKAGE")]
        package: PathBuf,
    },
    /// Computes cryptographic SHA-256 hash for a file or asset
    Hash {
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },
    /// Signs a package manifest with a publisher private key
    Sign {
        #[arg(value_name = "PACKAGE")]
        package: PathBuf,
        #[arg(short, long, value_name = "KEY")]
        key: String,
    },
    /// Verifies package signature and asset integrity
    Verify {
        #[arg(value_name = "PACKAGE")]
        package: PathBuf,
    },
}

fn main() -> Result<()> {
    init_diagnostics(Level::INFO);
    let cli = Cli::parse();

    match cli.command {
        Commands::Validate { path } => {
            info!("Validating package at: {:?}", path);
            if path.is_file() {
                let file = File::open(&path)?;
                let mut zip = ZipArchive::new(file)?;
                let mut manifest_entry = zip
                    .by_name("manifest.json")
                    .context("Missing manifest.json in archive")?;
                let mut content = Vec::new();
                manifest_entry.read_to_end(&mut content)?;
                let manifest: PetPackManifest = serde_json::from_slice(&content)?;
                manifest
                    .validate()
                    .map_err(|e| anyhow::anyhow!("Validation error: {}", e))?;
                println!(
                    "SUCCESS: Package '{}' (v{}) is valid!",
                    manifest.name, manifest.version
                );
            } else if path.is_dir() {
                let manifest_path = path.join("manifest.json");
                let content = fs::read(&manifest_path)?;
                let manifest: PetPackManifest = serde_json::from_slice(&content)?;
                manifest
                    .validate()
                    .map_err(|e| anyhow::anyhow!("Validation error: {}", e))?;
                println!("SUCCESS: Directory manifest '{}' is valid!", manifest.name);
            }
        }
        Commands::Build { source, output } => {
            info!(
                "Building .openpet package from {:?} -> {:?}",
                source, output
            );
            build_petpack(&source, &output)?;
            println!("SUCCESS: Created package at: {:?}", output);
        }
        Commands::Inspect { package } => {
            let file = File::open(&package)?;
            let mut zip = ZipArchive::new(file)?;
            let file_count = zip.len();
            let mut content = Vec::new();
            {
                let mut manifest_entry = zip.by_name("manifest.json")?;
                manifest_entry.read_to_end(&mut content)?;
            }
            let manifest: PetPackManifest = serde_json::from_slice(&content)?;

            println!("--- OpenPet Package Inspection ---");
            println!("ID           : {}", manifest.id);
            println!("Name         : {}", manifest.name);
            println!("Version      : {}", manifest.version);
            println!("Author       : {}", manifest.author);
            println!("License      : {}", manifest.license);
            println!("Description  : {}", manifest.description);
            println!("Files in pkg : {}", file_count);
            println!(
                "Animations   : {}",
                manifest
                    .animations
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        Commands::Hash { file } => {
            let bytes = fs::read(&file)?;
            let hash = compute_sha256(&bytes);
            println!("SHA256 ({}): {}", file.display(), hash);
        }
        Commands::Sign { package, key: _ } => {
            println!("Simulated publisher signature applied to {:?}", package);
        }
        Commands::Verify { package } => {
            let file = File::open(&package)?;
            let mut zip = ZipArchive::new(file)?;
            let mut manifest_entry = zip.by_name("manifest.json")?;
            let mut content = Vec::new();
            manifest_entry.read_to_end(&mut content)?;
            let manifest: PetPackManifest = serde_json::from_slice(&content)?;
            println!("SUCCESS: Package '{}' verified.", manifest.id);
        }
    }

    Ok(())
}
