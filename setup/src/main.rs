mod config;
mod console;
mod constants;
mod download;
mod install;
mod options;
mod update;

use anyhow::{anyhow, Context, Result};
use indoc::indoc;
use reqwest::Url;
use self::console::Console;
use self::install::Installer;
use self::options::{Options, Command};
use std::path::{Path, PathBuf};
use self::update::Updater;
use structopt::StructOpt;
use util::Location;

const BASE_URL: &str = "https://github.com/cluvio/agent/releases";

fn main() -> Result<()> {
    let opts = Options::from_args();

    if opts.version {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return Ok(())
    }

    tracing_subscriber::fmt()
        .with_env_filter(opts.log.unwrap_or_else(|| "setup=info".to_string()))
        .init();

    let base_url = Url::parse(BASE_URL).expect("base url");
    let mut console = Console::new();

    match opts.command {
        Some(Command::Install { version, location, directory }) => {
            let directory =
                if let Some(dir) = directory {
                    dir
                } else {
                    get_directory(&mut console)?
                };
            let mut installer = Installer::new(console, location, directory, version);
            installer.install(&base_url).context("Installation failed.")?
        }
        Some(Command::Update { version, directory }) => {
            let directory =
                if let Some(dir) = directory {
                    dir
                } else {
                    get_directory(&mut console)?
                };
            let mut updater = Updater::new(console, directory, version);
            updater.update(&base_url).context("Update failed.")?
        }
        None => {
            let answer = console.ask(indoc! {
                "Would you like to install [i] a new agent or update [u] an existing installation?: [i/u] "
            })?;
            match &*answer {
                "i" | "I" => {
                    let directory = get_directory(&mut console)?;
                    let mut installer = Installer::new(console, Location::Eu, directory, None);
                    installer.install(&base_url).context("Installation failed.")?
                },
                "u" | "U" => {
                    let directory = get_directory(&mut console)?;
                    let mut updater = Updater::new(console, directory, None);
                    updater.update(&base_url).context("Update failed.")?
                }
                other => return Err(anyhow!("Invalid input {:?}", other))
            }
        }
    }

    Ok(())
}

fn get_directory(console: &mut Console) -> Result<PathBuf> {
    let answer = console.ask("Please enter the installation directory: ")?;
    Ok(map_dir(&*answer))
}

#[cfg(unix)]
fn map_dir(s: &str) -> PathBuf {
    if let Some(rem) = s.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rem)
        }
    }
    Path::new(s).to_path_buf()
}

#[cfg(windows)]
fn map_dir(s: &str) -> PathBuf {
    Path::new(s).to_path_buf()
}