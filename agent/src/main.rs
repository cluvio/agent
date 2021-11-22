use agent::{self, Agent, Config, Options};
use structopt::StructOpt;
use util::exit;

#[tokio::main]
async fn main() {
    let opts = Options::from_args();

    if opts.version {
        println!("{}", agent::version().unwrap_or_else(exit("version")));
        return
    }

    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(opts.log.unwrap_or_else(|| "agent=info".to_string()));

    if opts.json {
        subscriber.json().init();
    } else {
        subscriber.init();
    }

    let cfg: Config = {
        let path = opts.config
            .ok_or_else(|| "missing config path".to_string())
            .unwrap_or_else(exit("config"));
        let mut cfg = config::Config::default();
        cfg.merge(config::File::from(path)).unwrap_or_else(exit("config"));
        cfg.merge(config::Environment::with_prefix("AGENT")).unwrap_or_else(exit("config"));
        cfg.try_into().unwrap_or_else(exit("config"))
    };

    let reason = Agent::new(cfg)
        .unwrap_or_else(exit("agent"))
        .go()
        .await;

    exit("agent was terminated by gateway")(reason)
}

