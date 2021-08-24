use anyhow::{anyhow, Context, Result};
use crate::config::{create_config, CONFIG_FILE};
use crate::console::Console;
use crate::constants::ARCHIVE_TEMPLATE;
use crate::download::{download, latest_version};
use crossterm::style::Stylize;
use indoc::formatdoc;
use reqwest::Url;
use semver::Version;
use std::ffi::OsStr;
use std::io::{self, BufReader};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use util::{base64, Location};

#[derive(Debug)]
pub struct Installer {
    console: Console,
    directory: PathBuf,
    location: Location,
    version: Option<Version>
}

impl Installer {
    pub fn new(console: Console, loc: Location, dir: PathBuf, version: Option<Version>) -> Self {
        Installer { console, directory: dir, location: loc, version }
    }

    pub fn install(&mut self, url: &Url) -> Result<()> {
        let version =
            if let Some(v) = &self.version {
                v.clone()
            } else {
                let v = latest_version(&mut self.console, url)
                    .with_context(|| format!("Failed to fetch version information from {}.", url))?;
                self.console.say(format!("Latest version found: {}\n", v.to_string().bold()))?;
                let answer = self.console.ask("Would you like to install this version? [Y/n]: ")?;
                if !answer.is_any_of(["", "y", "Y"]) {
                    return Ok(())
                }
                v
            };

        let archive = PathBuf::from(ARCHIVE_TEMPLATE.replace("<VERSION>", &version.to_string()));
        let temp = tempdir().context("Failed to create temporary directory.")?;
        let temp_path = temp.path();

        {
            let mut download_url = url.clone();
            download_url.path_segments_mut().expect("base url")
                .push("download")
                .push(&format!("v{}", version));

            download(&mut self.console, &temp_path, &archive, &download_url).context("Download failed.")?
        }

        while !self.directory.is_dir() {
            let answer = self.console.ask(format! {
                "The directory \"{}\" does not exist yet. Should I create it? [Y/n]: ",
                self.directory.to_string_lossy().bold()
            })?;
            if answer.is_any_of(["", "y", "Y"]) {
                fs::create_dir_all(&self.directory)
                    .with_context(|| format!("Failed to create directory {:?}", &self.directory))?;
                break
            }
            let answer = self.console.ask("Abort the installation? [Y/n]: ")?;
            if answer.is_any_of(["", "y", "Y"]) {
                return Ok(())
            }
        }

        let section = self.console.begin(format! {
            "Extracting {} to \"{}\" ...",
            archive.to_string_lossy().bold(),
            self.directory.to_string_lossy().bold()
        })?;
        extract(&temp_path, &temp_path.join(archive), &self.directory)?;
        section.end()?;

        let section = self.console.begin(format! {
            "Creating config file \"{}\" ...",
            self.directory.join(CONFIG_FILE).to_string_lossy().bold()
        })?;
        let pubkey = create_config(&self.directory, self.location).context("Failed to create config file.")?;
        section.end()?;

        self.console.print(formatdoc! {
            "{done}Please register the following agent key with cluvio.com:

                {key}

            Once registered, the agent can be run with:

                {agent} -c {config}
            \n",
            done = "Installation complete.\n\n".green().bold(),
            key = base64::encode(&pubkey).bold().yellow().on_black(),
            agent  = self.directory.join("cluvio-agent").to_string_lossy().bold(),
            config = self.directory.join(CONFIG_FILE).to_string_lossy().bold()
        })?;

        Ok(())
    }
}

pub fn extract(temp: &Path, archive: &Path, dir: &Path) -> Result<()> {
    if Some("xz") != archive.extension().and_then(OsStr::to_str) {
        return Err(anyhow!("Archive {:?} has no .xz file extension", archive))
    }
    let mut r = {
        let f = File::open(temp.join(archive)).with_context(|| format!("Failed to open archive {:?}", archive))?;
        xz2::bufread::XzDecoder::new(BufReader::new(f))
    };
    let path  = temp.join(archive.file_stem().expect("file has an extension"));
    let mut w = File::create(&path).with_context(|| format!("Failed to create {:?}", path))?;
    io::copy(&mut r, &mut w).context("Failed to move archive contents to destination")?;
    drop(w);
    let mut tar = {
        let f = File::open(&path).with_context(|| format!("Failed to open {:?}", path))?;
        tar::Archive::new(BufReader::new(f))
    };
    tar.unpack(dir).context("Failed to untar archive")?;
    Ok(())
}

