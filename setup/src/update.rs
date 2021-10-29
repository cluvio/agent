use anyhow::{anyhow, Context, Result};
use crate::console::Console;
use crate::constants::{AGENT, ARCHIVE_TEMPLATE};
use crate::download::{download, latest_version};
use crate::install::{self, CONFIG_FILE};
use crossterm::style::Stylize;
use reqwest::Url;
use semver::Version;
use std::process::Command;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::str;
use tempfile::tempdir;
use walkdir::WalkDir;

#[derive(Debug)]
pub struct Updater {
    console: Console,
    directory: PathBuf,
    version: Option<Version>
}

impl Updater {
    pub fn new(console: Console, dir: PathBuf, version: Option<Version>) -> Self {
        Updater { console, directory: dir, version }
    }

    pub fn update(&mut self, base_url: &Url) -> Result<()> {
        if !self.directory.is_dir() {
            return Err(anyhow!("{:?} is not a directory", self.directory))
        }

        let version =
            if let Some(v) = &self.version {
                v.clone()
            } else {
                let i = self.installed_version().context("Failed to get installed version.")?;
                self.console.say(format!("Currently installed version: {}\n", i.to_string().bold()))?;
                let v = latest_version(&mut self.console, base_url).context("Failed to fetch remote version information.")?;
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
            let mut download_url = base_url.clone();
            download_url.path_segments_mut().expect("base url")
                .push("download")
                .push(&format!("v{}", version));

            download(&mut self.console, temp_path, &archive, &download_url).context("Download failed.")?
        }

        let section = self.console.begin(format! {
            "Extracting {} to \"{}\" ...",
            archive.to_string_lossy().bold(),
            self.directory.to_string_lossy().bold()
        })?;
        install::extract(temp_path, &archive, temp_path).context("Failed to extract archive.")?;
        section.end()?;
        self.copy(temp_path, &[archive.as_os_str()])?;

        self.console.print(format!("{}\n", "Update complete.".green().bold()))?;

        Ok(())
    }

    fn copy(&mut self, source: &Path, ignore: &[&OsStr]) -> Result<()> {
        for entry in WalkDir::new(source).min_depth(1).max_depth(10) {
            let entry = entry.context("Failed to read directory entry.")?;
            let path = entry.path().strip_prefix(source).context("Failed to strip path prefix")?;
            if entry.file_type().is_dir() {
                fs::create_dir_all(self.directory.join(path))
                    .with_context(|| format!("Failed to creat directory {:?}", self.directory.join(path)))?;
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
            fs::copy(entry.path(), &dest)
                .with_context(|| format!("Failed to copy {:?} to {:?}", entry, dest))?;
        }
        Ok(())
    }

    fn installed_version(&mut self) -> Result<Version> {
        let section = self.console.begin("Checking installed version ...")?;
        let path = self.directory.join(AGENT);
        let out = Command::new(&path)
            .arg("--version")
            .output()
            .with_context(|| format!("Error executing {:?}.", path))?;
        if !out.status.success() {
            return Err(anyhow!("Failed to get version information from {:?}.", path));
        }
        let v = {
            let s = str::from_utf8(&out.stdout)?.trim();
            Version::parse(s).context("Invalid version string.")?
        };
        section.end()?;
        Ok(v)
    }
}
