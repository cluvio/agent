use crate::config::CONFIG_FILE;
use crate::console::Console;
use crate::constants::ARCHIVE_TEMPLATE;
use crate::download::{download, latest_version};
use crate::install;
use crate::error::Error;
use crossterm::style::Stylize;
use reqwest::Url;
use semver::Version;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use walkdir::WalkDir;

#[derive(Debug)]
pub struct Updater {
    console: Console,
    directory: PathBuf,
    version: Option<Version>
}

impl Updater {
    pub fn new(dir: PathBuf, version: Option<Version>) -> Self {
        Updater { console: Console::new(), directory: dir, version }
    }

    pub fn update(&mut self, base_url: &Url) -> Result<(), Error> {
        if !self.directory.is_dir() {
            self.console.say(format!("\"{}\" not found\n", self.directory.to_string_lossy().bold()))?;
            return Err(Error::UpdateFailed)
        }

        let version =
            if let Some(v) = &self.version {
                v.clone()
            } else {
                let v = latest_version(&mut self.console, base_url)?;
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
            let mut download_url = base_url.clone();
            download_url.path_segments_mut().expect("base url")
                .push("download")
                .push(&format!("v{}", version));

            download(&mut self.console, temp.path(), &archive, &download_url)?
        }

        install::extract(&temp.path().join(&archive), temp.path())?;
        fs::File::create(temp.path().join(CONFIG_FILE))?;
        self.copy(temp.path(), &[archive.as_os_str()])?;

        self.console.say(format!("Update complete\n"))?;

        Ok(())
    }

    fn copy(&mut self, source: &Path, ignore: &[&OsStr]) -> Result<(), Error> {
        for entry in WalkDir::new(source).min_depth(1).max_depth(10) {
            let entry = entry.map_err(|e| Error::Io(io::Error::new(io::ErrorKind::Other, e)))?;
            let path = entry.path()
                .strip_prefix(source)
                .map_err(|e| Error::Io(io::Error::new(io::ErrorKind::Other, e)))?;
            if entry.file_type().is_dir() {
                fs::create_dir_all(self.directory.join(path))?;
                continue
            }
            if ignore.contains(&entry.file_name()) {
                continue
            }
            let dest = {
                let mut new_path = self.directory.join(path);
                if path.as_os_str() == CONFIG_FILE && new_path.is_file() {
                    new_path.set_extension("toml.new");
                }
                new_path
            };
            fs::copy(entry.path(), dest)?;
        }
        Ok(())
    }
}
