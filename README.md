# Cluvio Agent

# Building and running an agent

Running an agent requires a configuration file which specifies among other
things the gateway server to connect to. To generate a fresh config file
one can use the `setup` tool:

```
> cargo run --bin setup -- config -o /tmp/config.toml
    Finished dev [unoptimized + debuginfo] target(s) in 0.05s
     Running `target/debug/setup config -o /tmp/config.toml`
In which location do you want to run the agent? [US/EU]: eu
```

This creates a new configuration against a production gateway instance in the
US or EU:

```
> cat /tmp/config.toml
# This is the key to register at cluvio.com:
agent-key = "HfHljrdm_Lrof1k6bvtMFgC7hdH5R64PmyCsNFTvE28"

# This key must not be shared with anyone:
secret-key = "PKDdhZR-PlJJy2wKiCVeiw993rtZlLFmswyxbZGzw4g"

[server]
host = "gateway.eu.cluvio.com"⏎
```

For testing against staging, the `host` value in the `[server]` section needs
to point to a staging server, e.g.

```
[server]
host = "gateway-staging.eu.cluvio.com"
```

Finally, to test against a local gateway instance one can provide host and port
and (if using a self-signed certificate), a trust anchor, e.g.

```
[server]
host  = "localhost"
port  = 7000
trust = '''
-----BEGIN CERTIFICATE-----
MIIBATCBtKADAgECAhQRZajw4FFtDMuGApP6tA1Sq4JDVTAFBgMrZXAwFDESMBAG
A1UEAwwJbG9jYWxob3N0MB4XDTIxMTAxMjEyMzU1NVoXDTIxMTExMTEyMzU1NVow
FDESMBAGA1UEAwwJbG9jYWxob3N0MCowBQYDK2VwAyEAiPzudDhawzS+Bc4AtURC
saMx4N6pKPyQLidpgvnijLqjGDAWMBQGA1UdEQQNMAuCCWxvY2FsaG9zdDAFBgMr
ZXADQQAyWF29mcwXP0Jbyk/0I5RL8OMgsFTXVg3/J1z6oPQnUtQLePKoHr/3K802
0kTeSGmLPxglCQ5fuDBHj+i24tgL
-----END CERTIFICATE-----
'''
```

`trust` corresponds to the self-signed certificate the gateway uses.
Given a config file, the agent can be run with

```
cargo run --bin agent -- -c /tmp/config.toml
```

For testing, it may be instructive to raise the log level, e.g.

```
cargo run --bin agent -- -c /tmp/config.toml -l agent=debug
```

