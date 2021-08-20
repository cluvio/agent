use crate::error::Error;
use indoc::indoc;
use sealed_boxes::PublicKey;
use std::fs;
use std::path::Path;
use util::{base64, Location};

pub const CONFIG_FILE: &str = "cluvio-agent.toml";

const HOST_TEMPLATE: &str = "ext.gateway-<LOCATION>.cluvio.com";
const CONFIG_TEMPLATE: &str = indoc! {r#"
    private-key      = "<PRIVATEKEY>"
    allowed-external = ["*.cluvio.com"]

    [control-server]
    host = "<HOST>"
    port = 9000
"#};

pub fn create_config<P>(dir: P, loc: Location) -> Result<PublicKey, Error>
where
    P: AsRef<Path>
{
    let skey = sealed_boxes::gen_secret_key();
    let sb64 = base64::encode(skey.to_bytes());
    let conf = CONFIG_TEMPLATE
        .replace("<PRIVATEKEY>", &sb64)
        .replace("<HOST>", &HOST_TEMPLATE.replace("<LOCATION>", &loc.to_string()));
    fs::write(dir.as_ref().join(Path::new(CONFIG_FILE)), conf.as_bytes())?;
    Ok(skey.public_key())
}

