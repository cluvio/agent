mod config;
mod console;
mod constants;
mod download;
mod error;
mod install;
mod options;
mod update;

use reqwest::Url;
use self::install::Installer;
use self::options::{Options, Command};
use self::update::Updater;
use structopt::StructOpt;
use util::exit;

const BASE_URL: &str = "https://github.com/cluvio/agent/releases";

fn main() {
    let opts = Options::from_args();

    tracing_subscriber::fmt()
        .with_env_filter(opts.log.unwrap_or_else(|| "setup=info".to_string()))
        .init();

    let base_url = Url::parse(BASE_URL).expect("base url");

    match opts.command {
        Command::Install { version, location, directory } => {
            let mut installer = Installer::new(location, directory, version);
            installer.install(&base_url).unwrap_or_else(exit("install"))
        }
        Command::Update { version, directory } => {
            let mut updater = Updater::new(directory, version);
            updater.update(&base_url).unwrap_or_else(exit("update"))
        }
    }
}

