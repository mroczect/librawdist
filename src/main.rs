use clap::{Parser, Subcommand};
use librawdist::{self, PackageManager, fetch::UreqClient, fs::RealFs, types::RawdistConfig};
use miette::{IntoDiagnostic, Result, WrapErr};
use std::path::PathBuf;
use std::process::ExitCode;

// ---------------------------------------------------------------------------
// CLI definition
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(
    name = "rawdist",
    version = env!("CARGO_PKG_VERSION"),
    about = "A robust package manager for rawssg",
    long_about = None
)]
struct Cli {
    /// Verbosity level (-v, -vv, ...)
    #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    /// Quiet mode (errors only)
    #[arg(short = 'q', long = "quiet", global = true, conflicts_with = "verbose")]
    quiet: bool,

    /// Path to the manifest file (rawssg-packages.toml)
    #[arg(
        short = 'm',
        long = "manifest",
        global = true,
        default_value = "rawssg-packages.toml"
    )]
    manifest: PathBuf,

    /// Path to the lockfile (Rawdist.lock)
    #[arg(
        short = 'l',
        long = "lockfile",
        global = true,
        default_value = "Rawdist.lock"
    )]
    lockfile: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a .rawdist package from a source directory
    Pack {
        #[arg(default_value = ".")]
        source_dir: PathBuf,
        #[arg(short = 'o', long = "output")]
        output: Option<PathBuf>,
    },
    /// Install a package from a .rawdist file or URL
    Install {
        source: String,
        #[arg(short = 't', long = "target")]
        target: Option<PathBuf>,
    },
    /// Remove an installed package
    Uninstall {
        package_name: String,
    },
    /// Verify the integrity of a .rawdist archive
    Verify {
        archive: PathBuf,
        #[arg(short = 'k', long = "keep")]
        keep: bool,
    },
    /// Download a .rawdist archive from a URL
    Fetch {
        url: String,
        #[arg(short = 'o', long = "output")]
        output: Option<PathBuf>,
    },
    /// List installed packages
    List,
    /// Show package information from a .rawdist archive or directory
    Info {
        path: PathBuf,
    },
}

// ---------------------------------------------------------------------------
// Logging setup
// ---------------------------------------------------------------------------

fn setup_logging(verbose: u8, quiet: bool) {
    let level = if quiet {
        log::LevelFilter::Error
    } else {
        match verbose {
            0 => log::LevelFilter::Warn,
            1 => log::LevelFilter::Info,
            2 => log::LevelFilter::Debug,
            _ => log::LevelFilter::Trace,
        }
    };

    let _ = env_logger::Builder::new()
        .filter_level(level)
        .format_timestamp(None)
        .format_target(false)
        .try_init();
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn looks_like_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}

fn validate_url(url: &str) -> Result<()> {
    if !looks_like_url(url) {
        return Err(miette::miette!(
            "Invalid URL '{}': must start with http:// or https://",
            url
        ));
    }
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or("");
    if rest.is_empty() {
        return Err(miette::miette!("Invalid URL '{}': missing host", url));
    }
    Ok(())
}

fn print_package_info(config: &RawdistConfig) {
    println!("Name        : {}", config.package.name);
    println!("Version     : {}", config.package.version);
    println!("Edition     : {}", config.edition);
    if let Some(desc) = &config.package.description {
        println!("Description : {}", desc);
    }
    println!("rawssg type : {}", config.rawssg.r#type);
    println!("Target dir  : {}", config.resolve_target_dir());
    if let Some(authors) = &config.package.authors {
        println!("Authors     : {}", authors.join(", "));
    }
    if let Some(license) = &config.package.license {
        println!("License     : {}", license);
    }
    if let Some(repo) = &config.package.repository {
        println!("Repository  : {}", repo);
    }
    println!("Include files: {:?}", config.files.include);
    if !config.files.exclude.is_empty() {
        println!("Exclude files: {:?}", config.files.exclude);
    }
}

// ---------------------------------------------------------------------------
// Main logic
// ---------------------------------------------------------------------------

fn run(cli: Cli) -> Result<()> {
    let fs = RealFs;
    let http = UreqClient;
    let manager = PackageManager::new(&fs, &http, cli.manifest.clone(), cli.lockfile.clone());

    match cli.command {
        Commands::Pack { source_dir, output } => {
            if !source_dir.exists() {
                return Err(miette::miette!(
                    "Source directory not found: {}",
                    source_dir.display()
                ));
            }
            if !source_dir.is_dir() {
                return Err(miette::miette!(
                    "Source path is not a directory: {}",
                    source_dir.display()
                ));
            }

            let config = RawdistConfig::load_from_dir(&fs, &source_dir)
                .wrap_err("Failed to load configuration from source directory")?;

            let out_path = output.unwrap_or_else(|| {
                let name = format!("{}-{}.rawdist", config.package.name, config.package.version);
                PathBuf::from(name)
            });

            if out_path.exists() {
                return Err(miette::miette!(
                    "Output file already exists: {}. Choose a different --output or remove it first.",
                    out_path.display()
                ));
            }

            manager
                .create(&source_dir, &out_path, &config)
                .wrap_err("Failed to create package")?;

            println!("Package created successfully: {}", out_path.display());
        }

        Commands::Install { source, target } => {
            if looks_like_url(&source) {
                validate_url(&source)?;
                manager
                    .install_from_url(&source, target.as_deref())
                    .wrap_err("Failed to install from URL")?;
            } else {
                let archive = PathBuf::from(&source);
                if !archive.exists() {
                    return Err(miette::miette!("File not found: {}", source));
                }
                if archive.is_dir() {
                    return Err(miette::miette!(
                        "Expected a .rawdist archive file, but '{}' is a directory",
                        source
                    ));
                }
                manager
                    .install(&archive, target.as_deref())
                    .wrap_err("Failed to install package")?;
            }
            println!("Package installed successfully.");
        }

        Commands::Uninstall { package_name } => {
            if package_name.trim().is_empty() {
                return Err(miette::miette!("Package name cannot be empty"));
            }
            manager
                .uninstall(&package_name)
                .wrap_err("Failed to uninstall package")?;
            println!("Package '{}' uninstalled successfully.", package_name);
        }

        Commands::Verify { archive, keep } => {
            if !archive.exists() {
                return Err(miette::miette!("Archive not found: {}", archive.display()));
            }
            let result = manager
                .verify(&archive, keep)
                .wrap_err("Verification failed")?;
            if let Some(path) = result {
                println!(
                    "Verification successful. Extracted content kept at: {}",
                    path.display()
                );
            } else {
                println!("Verification successful. Archive is valid.");
            }
        }

        Commands::Fetch { url, output } => {
            validate_url(&url)?;
            if let Some(ref out) = output {
                if out.exists() && out.is_dir() {
                    return Err(miette::miette!(
                        "Output path '{}' is a directory, expected a file path",
                        out.display()
                    ));
                }
            }
            let dest = output.as_deref();
            let path = librawdist::fetch_package(&fs, &http, &url, dest)
                .wrap_err("Failed to download package")?;
            println!("Package saved to: {}", path.display());
        }

        Commands::List => {
            let manifest = manager.list().wrap_err("Failed to read manifest")?;
            if manifest.packages.is_empty() {
                println!("No installed packages.");
            } else {
                println!("Installed packages:");
                for pkg in &manifest.packages {
                    println!(
                        "  {}@{} -> {}",
                        pkg.name,
                        pkg.version,
                        pkg.install_path.display()
                    );
                }
            }
        }

        Commands::Info { path } => {
            if path.is_dir() {
                let config = RawdistConfig::load_from_dir(&fs, &path)
                    .wrap_err("Failed to read configuration from directory")?;
                print_package_info(&config);
            } else if path.is_file() && path.extension().map_or(false, |e| e == "rawdist") {
                let extracted = librawdist::package::extract_to_temp(&fs, &path)
                    .wrap_err("Failed to extract archive")?;
                let config = RawdistConfig::load_from_dir(&fs, &extracted)
                    .wrap_err("Failed to read configuration from archive");
                let cleanup = std::fs::remove_dir_all(&extracted);
                let config = config?;
                cleanup
                    .into_diagnostic()
                    .wrap_err("Failed to clean up temporary directory")?;
                print_package_info(&config);
            } else if !path.exists() {
                return Err(miette::miette!("Path not found: {}", path.display()));
            } else {
                return Err(miette::miette!(
                    "Path must be a source directory or a .rawdist file, got: {}",
                    path.display()
                ));
            }
        }
    }

    Ok(())
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    setup_logging(cli.verbose, cli.quiet);

    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(report) => {
            eprintln!("{:?}", report);
            ExitCode::FAILURE
        }
    }
}
