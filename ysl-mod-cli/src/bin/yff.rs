use core::fmt;
use std::{ffi::OsStr, fs, path::PathBuf, process::ExitCode};

use anstream::{eprintln, println};
use anyhow::{Context, bail};
use clap::{Parser, Subcommand, ValueEnum};

const ERR_STYLE: anstyle::Style =
    anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Red)));
const MSG_STYLE: anstyle::Style =
    anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Green)));

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Template {
        #[arg(long)]
        to: PathBuf,
        #[arg(long)]
        name: String,
        #[arg(long, value_enum)]
        engine: Engine,
    },
    Pack {
        #[arg(long)]
        from: PathBuf,
        #[arg(long)]
        to: PathBuf,
    },
    Unpack {
        #[arg(long)]
        from: PathBuf,
        #[arg(long)]
        to: PathBuf,
    },
    Info {
        #[arg(long)]
        from: PathBuf,
    },
    Diff {
        #[arg(long)]
        from: PathBuf,
        #[arg(long)]
        to: PathBuf,
    },
}

#[derive(Debug, Clone, ValueEnum)]
pub enum Engine {
    Rhai,
    Lua,
}

impl fmt::Display for Engine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Engine::Rhai => write!(f, "rhai"),
            Engine::Lua => write!(f, "lua"),
        }
    }
}

impl From<Engine> for ysl_mod::package::Engine {
    fn from(value: Engine) -> Self {
        match value {
            Engine::Rhai => ysl_mod::package::Engine::Rhai,
            Engine::Lua => ysl_mod::package::Engine::Lua,
        }
    }
}

fn run_template(to: PathBuf, name: &str, engine: &ysl_mod::package::Engine) -> anyhow::Result<()> {
    let mut builder = ysl_mod::package::builder::PackageBuilder::new(name, *engine);
    for (path, data) in ysl_mod::export(&builder.build()?, to)? {
        let Some(parent) = path.parent() else {
            bail!("{path:?} is not a valid path");
        };
        fs::create_dir_all(parent)?;
        std::fs::write(&path, data).with_context(|| format!("Writing in {:?} failed", path))?
    }

    println!("{MSG_STYLE}Template success{MSG_STYLE:#}");
    Ok(())
}

fn run_pack(from: PathBuf, to: PathBuf) -> anyhow::Result<()> {
    //TODO: test if folder exist
    //TODO: test if folder is a folder
    //TODO: test if folder is empty

    let Some(name) = from.file_name().and_then(OsStr::to_str) else {
        bail!("Cannot extract name from {from:?}")
    };
    println!("{MSG_STYLE}Start packing {name}{MSG_STYLE:#}");
    let mut paths = vec![];

    let metadata_path = from.join("mod.toml");
    let rhai_folder = from.join("rhai");

    paths.push((metadata_path.to_owned(), std::fs::read(metadata_path)?));

    for entry in rhai_folder.read_dir()? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().is_some_and(|ext| ext == "rhai") {
            paths.push((path.to_owned(), std::fs::read(path)?));
        }
    }

    let package = ysl_mod::import(paths)?;
    let data = ysl_mod::write(&package, 0)?;
    std::fs::write(to.join(format!("{name}.yff")), data)?;

    println!("{MSG_STYLE}Packing {name} succeed{MSG_STYLE:#}");
    Ok(())
}

fn run_unpack(from: PathBuf, to: PathBuf) -> anyhow::Result<()> {
    let content = std::fs::read(&from)?;
    let package = ysl_mod::read(&content)?;

    for (path, data) in ysl_mod::export(&package, to)? {
        let Some(parent) = path.parent() else {
            bail!("{path:?} is not a valid path");
        };
        fs::create_dir_all(parent)?;
        std::fs::write(&path, data).with_context(|| format!("Writing in {:?} failed", path))?
    }

    println!("{MSG_STYLE}Unpack success{MSG_STYLE:#}");
    Ok(())
}

fn run_info(from: PathBuf) -> anyhow::Result<()> {
    let package = ysl_mod::read(&std::fs::read(&from)?)?;

    println!("{package:#?}");
    Ok(())
}

fn run_diff(from: PathBuf, to: PathBuf) -> anyhow::Result<()> {
    let package_from = ysl_mod::read(&std::fs::read(&from)?)?;
    let package_to = ysl_mod::read(&std::fs::read(&to)?)?;

    assert_eq!(package_from, package_to);
    Ok(())
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Template { to, name, engine } => run_template(to, &name, &engine.into())?,
        Commands::Pack { from, to } => run_pack(from, to)?,
        Commands::Unpack { from, to } => run_unpack(from, to)?,
        Commands::Info { from } => run_info(from)?,
        Commands::Diff { from, to } => run_diff(from, to)?,
    }
    Ok(())
}

fn main() -> ExitCode {
    if let Err(e) = run() {
        eprintln!("{ERR_STYLE}Error: {e:#}{ERR_STYLE:#}");

        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
