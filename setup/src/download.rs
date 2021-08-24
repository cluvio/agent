use anyhow::{anyhow, Result};
use crossterm::style::Stylize;
use reqwest::Url;
use semver::Version;
use crate::console::Console;
use std::fs;
use std::path::Path;

pub fn download(console: &mut Console, dir: &Path, file: &Path, url: &Url) -> Result<()> {
    let console = console.begin(format!("Downloading {} from {} ...", file.to_string_lossy().bold(), url))?;
    let mut url = url.clone();
    url.path_segments_mut().expect("base url").push(&file.to_string_lossy());
    let res = reqwest::blocking::get(url)?;
    if !res.status().is_success() {
        return Err(anyhow!("Invalid HTTP status: {}", res.status()))
    }
    fs::write(dir.join(file), &res.bytes()?)?;
    console.end()?;
    Ok(())
}

pub fn latest_version(console: &mut Console, url: &Url) -> Result<Version> {
    let console = console.begin("Checking latest version ...")?;
    let mut url = url.clone();
    url.path_segments_mut().expect("base url").push("latest");
    let client = reqwest::blocking::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let res = client.get(url).send()?;
    if !res.status().is_redirection() {
        return Err(anyhow!("Response status {} is not a redirect.", res.status()))
    }
    let url: Url = res.headers()
        .get("location")
        .ok_or(anyhow!("Missing location header"))?
        .to_str()
        .map_err(|e| anyhow::Error::from(e).context("Invalid location header"))?
        .parse()
        .map_err(|e| anyhow::Error::from(e).context("Invalid location URL"))?;
    let sgs = url.path_segments().and_then(|s| s.rev().next()).ok_or(anyhow!("Missing URL path segments"))?;
    let ver = Version::parse(sgs.strip_prefix("v").ok_or(anyhow!("Failed to parse URL as version"))?)?;
    console.end()?;
    Ok(ver)
}

