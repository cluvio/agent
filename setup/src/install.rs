use crate::config::{create_config, CONFIG_FILE};
use crate::console::Console;
use crate::constants::ARCHIVE_TEMPLATE;
use crate::download::{download, latest_version};
use crate::error::Error;
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
    pub fn new(loc: Location, dir: PathBuf, version: Option<Version>) -> Self {
        Installer { console: Console::new(), directory: dir, location: loc, version }
    }

    pub fn install(&mut self, url: &Url) -> Result<(), Error> {
        let version =
            if let Some(v) = &self.version {
                v.clone()
            } else {
                let v = latest_version(&mut self.console, url)?;
                self.console.say(format!("Latest version found: {}\n", v.to_string().bold()))?;
                let answer = self.console.ask("Would you like to download this version? [Y/n]: ")?;
                if !answer.is_any_of(["", "y", "Y"]) {
                    return Ok(())
                }
                v
            };

        let archive = PathBuf::from(ARCHIVE_TEMPLATE.replace("<VERSION>", &version.to_string()));
        let temp = tempdir()?;

        {
            let mut download_url = url.clone();
            download_url.path_segments_mut().expect("base url")
                .push("download")
                .push(&format!("v{}", version));

            download(&mut self.console, temp.path(), &archive, &download_url)?
        }

        while !self.directory.is_dir() {
            let answer = self.console.ask(format! {
                "The directory \"{}\" does not exist yet. Should I create it? [Y/n]: ",
                self.directory.to_string_lossy().bold()
            })?;
            if answer.is_any_of(["", "y", "Y"]) {
                fs::create_dir_all(&self.directory)?;
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
        extract(&temp.path().join(archive), &self.directory)?;
        section.end()?;

        let section = self.console.begin(format! {
            "Creating config file \"{}\" ...",
            self.directory.join(CONFIG_FILE).to_string_lossy().bold()
        })?;
        let pubkey = create_config(&self.directory, self.location)?;
        section.end()?;

        self.console.say(formatdoc! {
            "Installation complete. Please register the following agent key with cluvio.com: {key}\n",
            key = base64::encode(&pubkey).blue().bold()
        })?;
        self.console.say(formatdoc! {
            "Once registered, the agent can be run as: {agent} -c {config}\n",
            agent  = self.directory.join("cluvio-agent").to_string_lossy().bold(),
            config = self.directory.join(CONFIG_FILE).to_string_lossy().bold()
        })?;

        Ok(())
    }
}

pub fn extract(archive: &Path, dir: &Path) -> Result<(), Error> {
    if Some("xz") != archive.extension().and_then(OsStr::to_str) {
        let name = archive.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "n/a".to_string());
        return Err(Error::NoXzExt(name))
    }
    let mut r = xz2::bufread::XzDecoder::new(BufReader::new(File::open(archive)?));
    let path  = Path::new(archive.file_stem().expect("file has an extension"));
    let mut w = File::create(path)?;
    io::copy(&mut r, &mut w)?;
    drop(w);
    let mut tar = tar::Archive::new(BufReader::new(File::open(path)?));
    tar.unpack(dir)?;
    Ok(())
}

