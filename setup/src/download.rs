use crossterm::style::Stylize;
use reqwest::Url;
use semver::Version;
use crate::console::Console;
use crate::error::Error;
use std::fs;
use std::path::Path;

pub fn download(console: &mut Console, dir: &Path, file: &Path, url: &Url) -> Result<(), Error> {
    let console = console.begin(format!("Downloading {} from {} ...", file.to_string_lossy().bold(), url))?;
    let mut url = url.clone();
    url.path_segments_mut().expect("base url").push(&file.to_string_lossy());
    let res = reqwest::blocking::get(url)?;
    if !res.status().is_success() {
        return Err(Error::Status(res.status()))
    }
    fs::write(dir.join(file), &res.bytes()?)?;
    console.end()?;
    Ok(())
}

pub fn latest_version(console: &mut Console, url: &Url) -> Result<Version, Error> {
    let console = console.begin("Checking latest version ...")?;
    let mut url = url.clone();
    url.path_segments_mut().expect("base url").push("latest");
    let client = reqwest::blocking::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let res = client.get(url).send()?;
    if !res.status().is_redirection() {
        return Err(Error::NoVersion)
    }
    let url: Url = res.headers()
        .get("location")
        .ok_or(Error::NoVersion)?
        .to_str()
        .map_err(|_| Error::NoVersion)?
        .parse()
        .map_err(|_| Error::NoVersion)?;
    let sgs = url.path_segments().and_then(|s| s.rev().next()).ok_or(Error::NoVersion)?;
    let ver = Version::parse(sgs.strip_prefix("v").ok_or(Error::NoVersion)?)?;
    console.end()?;
    Ok(ver)
}

