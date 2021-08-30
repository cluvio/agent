use anyhow::{Context, Result};
use crate::console::Console;
use crossterm::style::Stylize;
use std::fs;
use std::path::Path;

#[derive(Debug, Copy, Clone)]
pub enum Outcome {
    Ready,
    Abort
}

pub fn create_dir(console: &mut Console, dir: &Path) -> Result<Outcome> {
    while !dir.is_dir() {
        let answer = console.ask(format! {
            "The directory \"{}\" does not exist yet. Should I create it? [Y/n]: ",
            dir.to_string_lossy().bold()
        })?;
        if answer.is_any_of(["", "y", "Y"]) {
            fs::create_dir_all(dir).with_context(|| format!("Failed to create directory {:?}", dir))?;
            break
        }
        let answer = console.ask("Abort the installation? [Y/n]: ")?;
        if answer.is_any_of(["", "y", "Y"]) {
            return Ok(Outcome::Abort)
        }
    }
    Ok(Outcome::Ready)
}

