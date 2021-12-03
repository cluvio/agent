# Cluvio Agent

A Cluvio Agent establishes connectivity between the Cluvio data centre and the user's
database. It maintains a single connection to Cluvio (using multiplexing to enable the
concurrent transfer of independent data streams) and creates connections to databases
when necessary. It is an alternative to running an SSH-Server or allowing inbound
connections to the database.

Each agent must be registered with a Cluvio account. It is identified with an ID, created
as part of the registration. A single agent can provide access to multiple databases. It
connects to Cluvio over TCP and TLS, verifiying the authenticity of the server, and it
authenticates itself to Cluvio as part of the connection establishment. Firewalls do not
need to allow inbound connections, it is sufficient if the agent is able to create an
outbound connection to Cluvio. Security aspects are discussed in more detail in
[SECURITY.md](/SECURITY.md).

## Installation

Pre-built binaries for Linux, MacOS and Windows are provided on GitHub at
https://github.com/cluvio/agent/releases. Before installing an agent a configuration
file should be retrieved from Cluvio at https://app.cluvio.com/settings/datasources/new.
An installed agent will attempt to find the configuration file named `cluvio-agent.toml`
at various platform-dependent file system locations:

### Linux

1. Next to the installed executable. For example if ther agent is installed as
`$HOME/cluvio/cluvio-agent` it will try to load `$HOME/cluvio/cluvio-agent.toml`.
2. In `$XDG_CONFIG_HOME` or `$HOME/.config`.
3. In `/etc`.

### MacOS

1. Next to the installed executable. For example if ther agent is installed as
`$HOME/cluvio/cluvio-agent` it will try to load `$HOME/cluvio/cluvio-agent.toml`.
2. In `$HOME/Library/Application Support`.
3. In `/etc`

### Windows

1. In `FOLDERID_RoamingAppData`, e.g. `C:\Users\JohnDoe\AppData\Roaming`
2. Next to the installed executable, e.g. `C:\Program Files\Cluvio`

*Please note that configuration files should not be shared between multiple agents.* If
you want to run multiple agents, download separate configurations for each installation.

## Running an agent

When the agent is started and successfully loads a configuration file it will attempt to
connect to Cluvio, authenticate itself and run indefinitely. If the connection is
interrupted, the agent will try to establish it again. As a command-line application
the agent accepts various parameters. `cluvio-agent --help` prints a quick summary. Some
options are:

- __`-c`__ | __`--config`__ accepts a path to a configuration file. This takes precedence
over the locations mentioned above which are checked.
- __`-l`__ | __`--log`__ specifies an explicit log level. The following log levels are used
for printing log messages to the console: `TRACE`, `DEBUG`, `INFO`, `WARN`, `ERROR`. By
default log messages on level `TRACE` and `DEBUG` are not visible. To include for instance
debug messages, the agent can be invoked with `--log debug`. The messages are also scoped
to various modules. To only see log messages from level `DEBUG` or higher from the agent
one could use `--log agent=debug`.
- __`-j`__ | __`--json`__ switches the log format to JSON. By default a human-friendly log
format is used. If the logs are processed by other programmes a more structured format may
be useful which is what `--json` provides.

### Running the agent as a service.

#### Linux

On Linux, the RPM and DEB archives include a unit file for systemd. Installation enables the
agent, but does not yet start the service. The unit file can be found in
[scripts/linux/cluvio-agent.service](/scripts/linux/cluvio-agent.service). Once the
configuration has been retrieved from Cluvio the usual `systemctl` commands can be used to
start, stop or inspect the agent, e.g. `systemctl status cluvio-agent.service`. Logs can
be found via journalctl, e.g. `journalctl -u cluvio-agent.service`.

