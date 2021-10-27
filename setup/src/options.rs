use semver::Version;
use std::path::PathBuf;
use structopt::StructOpt;
use util::Location;

/// Command-line options.
#[derive(Debug, StructOpt)]
#[non_exhaustive]
#[structopt(name = "cluvio-setup")]
pub struct Options {
    #[structopt(long)]
    pub log: Option<String>,

    /// Show version information.
    #[structopt(long)]
    pub version: bool,

    #[structopt(subcommand)]
    pub command: Option<Command>
}

#[derive(Debug, StructOpt)]
pub enum Command {
    /// Install the Cluvio agent.
    Install {
        /// The agent installation directory.
        #[structopt(short, long)]
        directory: Option<PathBuf>,

        /// The location this agent should use.
        #[structopt(short, long)]
        location: Option<Location>,

        /// Install a particular version.
        #[structopt(short, long)]
        version: Option<Version>
    },

    /// Update a previously installed Cluvio agent.
    Update {
        /// The agent installation directory to update.
        #[structopt(short, long)]
        directory: Option<PathBuf>,

        /// Install a particular version.
        #[structopt(short, long)]
        version: Option<Version>
    }
}

