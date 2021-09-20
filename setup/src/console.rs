use crossterm::style::Stylize;
use crossterm::{ExecutableCommand, cursor};
use std::io::{self, Write};
use std::ops::{Deref, DerefMut};

#[derive(Debug, PartialEq, Eq)]
pub struct Answer<'a>(&'a str);

impl<'a> Answer<'a> {
    pub fn is_any_of(&self, items: impl IntoIterator<Item = &'a str>) -> bool {
        for s in items {
            if s == self.0 {
                return true
            }
        }
        false
    }
}

impl Deref for Answer<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

#[derive(Debug)]
pub struct Console {
    stdin: io::Stdin,
    stdout: io::Stdout,
    answer: String
}

impl Console {
    pub fn new() -> Self {
        Console {
            stdin: io::stdin(),
            stdout: io::stdout(),
            answer: String::new()
        }
    }

    pub fn ask<S: AsRef<str>>(&mut self, msg: S) -> io::Result<Answer<'_>> {
        self.answer.clear();
        self.stdout.write_all(msg.as_ref().as_bytes())?;
        self.stdout.flush()?;
        self.stdin.read_line(&mut self.answer)?;
        Ok(Answer(self.answer.trim()))
    }

    pub fn print<S: AsRef<str>>(&mut self, msg: S) -> io::Result<()> {
        say(&mut self.stdout, msg)
    }

    pub fn say<S: AsRef<str>>(&mut self, msg: S) -> io::Result<()> {
        say(&mut self.stdout, msg)
    }

    pub fn begin<S: AsRef<str>>(&mut self, msg: S) -> io::Result<Section<'_>> {
        say(&mut self.stdout, format!("[    ] {}", msg.as_ref()))?;
        Ok(Section(self, true))
    }
}

pub struct Section<'a>(&'a mut Console, bool);

impl Section<'_> {
    pub fn end(mut self) -> io::Result<()> {
        self.0.stdout.execute(cursor::SavePosition)?;
        self.0.stdout.execute(cursor::MoveToColumn(3))?;
        say(&mut self.0.stdout, format!("{}", "OK".green()))?;
        self.0.stdout.execute(cursor::RestorePosition)?;
        println!();
        self.1 = false;
        Ok(())
    }
}

impl Drop for Section<'_> {
    fn drop(&mut self) {
        if self.1 {
            let _ = self.0.stdout.execute(cursor::SavePosition);
            let _ = self.0.stdout.execute(cursor::MoveToColumn(3));
            let _ = say(&mut self.0.stdout, format!("{}", "ER".red()));
            let _ = self.0.stdout.execute(cursor::RestorePosition);
            println!();
        }
    }
}

impl Deref for Section<'_> {
    type Target = Console;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl DerefMut for Section<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

fn say<S: AsRef<str>>(stdout: &mut io::Stdout, msg: S) -> io::Result<()> {
    stdout.write_all(msg.as_ref().as_bytes())?;
    stdout.flush()
}
