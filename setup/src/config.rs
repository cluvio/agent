use anyhow::{Context, Result};
use indoc::indoc;
use sealed_boxes::PublicKey;
use std::fs;
use std::path::Path;
use util::{base64, Location};

pub const CONFIG_FILE: &str = "cluvio-agent.toml";

const HOST_TEMPLATE: &str = "gateway-<LOCATION>.cluvio.com";
const CONFIG_TEMPLATE: &str = indoc! {r#"
    # This is the key to register at cluvio.com:
    agent-key  = "<AGENT_KEY>"

    # This key must not be shared with anyone:
    secret-key = "<SECRET_KEY>"

    [server]
    host = "<HOST>"
    port = 9000
"#};

pub fn create_config<P>(dir: P, loc: Location) -> Result<PublicKey>
where
    P: AsRef<Path>
{
    let skey = sealed_boxes::gen_secret_key();
    let pb64 = base64::encode(skey.public_key().as_bytes());
    let sb64 = base64::encode(skey.to_bytes());
    let conf = CONFIG_TEMPLATE
        .replace("<AGENT_KEY>", &pb64)
        .replace("<SECRET_KEY>", &sb64)
        .replace("<HOST>", &HOST_TEMPLATE.replace("<LOCATION>", &loc.to_string()));
    let file = dir.as_ref().join(Path::new(CONFIG_FILE));
    fs::write(&file, conf.as_bytes()).with_context(|| format!("Failed to write to {:?}", file))?;
    Ok(skey.public_key())
}

