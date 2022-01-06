use cluvio_agent::{self, Agent, Config, Options};
use directories::BaseDirs;
use std::env;
use std::path::{Path, PathBuf};
use structopt::StructOpt;
use util::{base64, exit};

const CONFIG_FILE_NAME: &str = "cluvio-agent.toml";

#[tokio::main]
async fn main() {
    let opts = Options::from_args();

    if opts.version {
        println!("{}", cluvio_agent::version().unwrap_or_else(exit("version")));
        return
    }

    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(opts.log.unwrap_or_else(|| "cluvio_agent=info".to_string()));

    if opts.json {
        subscriber.json().init();
    } else {
        subscriber.init();
    }

    if opts.gen_keypair {
        print_keypair();
        return
    }

    let cfg: Config = {
        let path = opts.config
            .or_else(find_config)
            .ok_or_else(|| concat!("see `", env!("CARGO_PKG_NAME"), " --help` for details").to_string())
            .unwrap_or_else(exit("config file not found"));
        log::info!(?path, "configuration");
        let mut cfg = config::Config::default();
        cfg.merge(config::File::from(path)).unwrap_or_else(exit("config"));
        cfg.merge(config::Environment::with_prefix("CLUVIO_AGENT").separator("_")).unwrap_or_else(exit("config"));
        cfg.try_into().unwrap_or_else(exit("config"))
    };

    let reason = Agent::new(cfg)
        .unwrap_or_else(exit("agent"))
        .go()
        .await;

    exit("agent was terminated by gateway")(reason)
}

/// Print a newly generated keypair to stdout.
fn print_keypair() {
    let s = sealed_boxes::gen_secret_key();
    let p = base64::encode(s.public_key().as_bytes());
    let s = base64::encode(s.to_bytes());
    println!("public-key: {}\nsecret-key: {}", p, s)
}

/// Try to find the config file in certain well-known locations.
fn find_config() -> Option<PathBuf> {
    fn exe_config() -> Option<PathBuf> {
        if let Ok(mut this) = env::current_exe() {
            this.pop();
            let cfg = this.join(CONFIG_FILE_NAME);
            if cfg.is_file() {
                return Some(cfg)
            }
        }
        None
    }
    fn usr_config() -> Option<PathBuf> {
        if let Some(base) = BaseDirs::new() {
            let cfg =
                if cfg!(target_os = "macos") {
                    base.home_dir().join(CONFIG_FILE_NAME)
                } else {
                    base.config_dir().join(CONFIG_FILE_NAME)
                };
            if cfg.is_file() {
                return Some(cfg)
            }
        }
        None
    }
    fn sys_config() -> Option<PathBuf> {
        let cfg = Path::new("/etc").join(CONFIG_FILE_NAME);
        if cfg.is_file() {
            return Some(cfg)
        }
        None
    }

    if cfg!(unix) {
        exe_config().or_else(usr_config).or_else(sys_config)
    } else if cfg!(windows) {
        usr_config().or_else(exe_config)
    } else {
        None
    }
}

