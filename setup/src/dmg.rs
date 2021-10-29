use crate::constants::AGENT;
use std::fs;
use std::io;
use std::path::Path;
use std::process::{Command, Output};
use std::str;

pub fn extract<P: AsRef<Path>>(dmg: P, to: P) -> io::Result<()> {
    let dmg_str = dmg.as_ref().to_str().ok_or_else(|| invalid_input("not a utf-8 path"))?;
    let output = Command::new("hdiutil")
        .args(&["attach", dmg_str, "-readonly"])
        .output()?;
    ensure_success(&output, "hdiutil attach failed")?;
    let mount = parse_mount_point(&output.stdout)?;
    fs::copy(Path::new(mount).join(AGENT), to)?;
    let output = Command::new("hdiutil")
        .args(&["detach", mount])
        .output()?;
    ensure_success(&output, "hdiutil detach failed")
}

fn parse_mount_point(bytes: &[u8]) -> io::Result<&str> {
    let b = bytes.rsplit(|&b| b == b'\t')
        .next()
        .ok_or_else(|| invalid_input("missing tab character in hdiutil output"))?;
    str::from_utf8(b).map_err(|e| invalid_input(format!("mount point not a utf-8 string: {}", e)))
}

fn invalid_input<E: Into<Box<dyn std::error::Error + Send + Sync>>>(e: E) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, e)
}

fn ensure_success(out: &Output, msg: &str) -> io::Result<()> {
    if !out.status.success() {
        if let Some(code) = out.status.code() {
            return Err(io::Error::from_raw_os_error(code))
        } else {
            return Err(io::Error::new(io::ErrorKind::Other, msg))
        }
    }
    Ok(())
}
