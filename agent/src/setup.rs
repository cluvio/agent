use sealed_boxes::PublicKey;
use std::fs;
use std::io;
use std::path::Path;
use util::base64;

const DEFAULT_CONFIG: &str = "\
private-key      = \"<PRIVATEKEY>\"
allowed-external = [\"*.cluvio.com\"]

[control-server]
host = \"<HOST>\"
port = 9000
";

const CONTROL_HOST: Option<&str> = option_env!("CLUVIO_SERVER");
const DEFAULT_CONTROL_HOST: &str = "ext.gateway-eu.cluvio.com";

pub fn setup<P: AsRef<Path>>(path: P) -> io::Result<PublicKey> {
    let skey = sealed_boxes::gen_secret_key();
    let sb64 = base64::encode(skey.to_bytes());
    let conf = DEFAULT_CONFIG
        .replace("<PRIVATEKEY>", &sb64)
        .replace("<HOST>", CONTROL_HOST.unwrap_or(DEFAULT_CONTROL_HOST));
    fs::write(path, conf.as_bytes())?;
    Ok(skey.public_key())
}

#[cfg(test)]
mod tests {
    use crate::Config;
    use super::{DEFAULT_CONFIG, DEFAULT_CONTROL_HOST};

    #[test]
    fn valid_default_config() {
        let s = sealed_boxes::gen_secret_key();
        let b = util::base64::encode(s.to_bytes());
        let c = DEFAULT_CONFIG.replace("<PRIVATEKEY>", &b).replace("<HOST>", DEFAULT_CONTROL_HOST);
        toml::from_slice::<Config>(c.as_bytes()).expect("valid config");
    }
}
