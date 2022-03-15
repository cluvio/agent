# Cluvio Agent

A Cluvio agent establishes connectivity between the Cluvio servers and a user's
database. Towards Cluvio, it maintains a single, multiplexed connection which enables
the concurrent transfer of independent data streams. Towards a user's database, it
creates connections as necessary. Running a Cluvio agent in front of a database is an
alternative to running an SSH server or allowing inbound connections to the database
from the internet.

Each Cluvio agent must be registered with a Cluvio account. To that end, each agent is identified by
a unique ID, assigned when creating a new agent in the Cluvio application. A single agent can
provide access to multiple databases. It connects to Cluvio over TCP with TLS, verifying the
authenticity of the server, and it authenticates itself to Cluvio as part of the connection
establishment. Firewalls do not need to allow inbound connections - it is sufficient if the agent is
able to create outbound connections to Cluvio and to the databases. Security aspects are discussed
in more detail in [SECURITY.md](/SECURITY.md).

## Configuration

To start an agent after installation, a configuration file is needed. This file is obtained from the
Cluvio application as a download when creating a new agent in the UI.  On startup, if not explicitly
specified via the `--config` option (more on the available options further below), an agent will
attempt to find the configuration file named `cluvio-agent.toml` at various platform-dependent file
system locations:

### Linux

1. Next to the installed executable. For example, if the agent is installed as
`$HOME/cluvio/cluvio-agent` it will try to load `$HOME/cluvio/cluvio-agent.toml`.
2. In `$XDG_CONFIG_HOME` or `$HOME/.config`.
3. In `/etc`.

### MacOS

1. Next to the installed executable. For example, if the agent is installed as
`$HOME/cluvio/cluvio-agent` it will try to load `$HOME/cluvio/cluvio-agent.toml`.
2. In `$HOME`.
3. In `/etc`

### Windows

1. In `FOLDERID_RoamingAppData`, e.g. `C:\Users\JohnDoe\AppData\Roaming`
2. Next to the installed executable, e.g. `C:\Program Files\Cluvio`

> **NOTE**: A configuration file can only be used by one running agent at a time.
> The file contains a secret key that uniquely identifies the agent and the Cluvio
> servers reject multiple connections from the same agent.

## Installation

Pre-built binaries for Linux, MacOS and Windows are provided on GitHub at
https://github.com/cluvio/agent/releases.

### MacOS

For users of [homebrew][1] a custom tap is available at https://github.com/cluvio/homebrew-tools.
The agent can be installed with `brew install cluvio/homebrew-tools/cluvio-agent`.

## Running an agent

When the agent is started and successfully loads a configuration file it will attempt to
connect to Cluvio, authenticate itself and run indefinitely. If the connection is
interrupted, the agent will try to establish it again. As a command-line application
the agent accepts various parameters. `cluvio-agent --help` prints a quick summary. Some
of the options are:

- __`-c`__ | __`--config`__ accepts a path to a configuration file. This takes precedence
over the locations mentioned above which are checked for a `cluvio-agent.toml`.
- __`-l`__ | __`--log`__ specifies an explicit log level. The following log levels are used
for printing log messages to the console: `trace`, `debug`, `info`, `warn`, `error`. By
default log messages on level `trace` and `debug` are not shown. To include for instance
debug messages, the agent can be invoked with `--log debug`. The messages are also scoped
to various modules. To only see log messages from level `debug` or higher from the agent
one could use `--log agent=debug`.
- __`-j`__ | __`--json`__ switches the log format to JSON. By default a human-friendly log
format is used. If the logs are processed by other programmes a more structured format may
be useful which is what `--json` provides.

### Running the agent as a service

#### Linux

On Linux, the RPM and DEB archives include a unit file for `systemd`. Installation enables the
agent, but does not yet start the service. The unit file can also be found at
[scripts/linux/cluvio-agent.service](/scripts/linux/cluvio-agent.service). Once the
configuration has been retrieved from Cluvio, the usual `systemctl` commands can be used to
start, stop or inspect the agent, e.g. `systemctl status cluvio-agent.service`. Logs can
be seen via `journalctl`, e.g. `journalctl -u cluvio-agent.service`.

#### MacOS

If [homebrew][1] is used for installation, the agent can be managed with the `services`
subcommand, e.g. `brew services start cluvio-agent`.

[1]: https://brew.sh/
