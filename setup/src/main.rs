mod config;
mod console;
mod constants;
mod dmg;
mod download;
mod install;
mod options;
mod update;
mod util;

use anyhow::{anyhow, Context, Result};
use indoc::indoc;
use reqwest::Url;
use self::console::Console;
use self::install::Installer;
use self::options::{Options, Command};
use std::path::{Path, PathBuf};
use std::process::exit;
use self::update::Updater;
use self::util::{create_dir, Outcome};
use structopt::StructOpt;
use ::util::Location;

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
            let location =
                if let Some(loc) = location {
                    loc
                } else {
                    get_location(&mut console)?
                };
            let directory =
                if let Some(dir) = directory {
                    dir
                } else {
                    get_directory(&mut console, true)?
                };
            let mut installer = Installer::new(console, location, directory, version);
            installer.install(&base_url).context("Installation failed.")?
        }
        Some(Command::Update { version, directory }) => {
            let directory =
                if let Some(dir) = directory {
                    dir
                } else {
                    get_directory(&mut console, false)?
                };
            let mut updater = Updater::new(console, directory, version);
            updater.update(&base_url).context("Update failed.")?
        }
        Some(Command::Config { output, location }) => {
            let location =
                if let Some(loc) = location {
                    loc
                } else {
                    get_location(&mut console)?
                };
            config::create_config(&output, location)?;
        }
        None => {
            let answer = console.ask(indoc! {
                "Would you like to install [i] a new agent or update [u] an existing installation? [i/u]: "
            })?;
            match &*answer {
                "i" | "I" => {
                    let location  = get_location(&mut console)?;
                    let directory = get_directory(&mut console, true)?;
                    let mut installer = Installer::new(console, location, directory, None);
                    installer.install(&base_url).context("Installation failed.")?
                }
                "u" | "U" => {
                    let directory = get_directory(&mut console, false)?;
                    let mut updater = Updater::new(console, directory, None);
                    updater.update(&base_url).context("Update failed.")?
                }
                other => return Err(anyhow!("Invalid input {:?}", other))
            }
        }
    }

    Ok(())
}

fn get_directory(console: &mut Console, create: bool) -> Result<PathBuf> {
    let answer = console.ask("Please enter the installation directory: ")?;
    let dir = map_dir(&*answer);
    if !dir.is_dir() {
        if !create {
            return Err(anyhow!("{:?} is not a directory.", dir))
        }
        match create_dir(console, &dir)? {
            Outcome::Ready => {}
            Outcome::Abort => exit(0)
        }
    }
    Ok(dir)
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

fn get_location(console: &mut Console) -> Result<Location> {
    let loc = console.ask("In which location do you want to run the agent? [US/EU]: ")?.parse()?;
    Ok(loc)
}

