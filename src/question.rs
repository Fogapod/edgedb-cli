use std::borrow::Cow;
use std::io::{stdin, BufRead};

use rustyline::{Editor, Config};

use anyhow::Context;


pub struct Numeric<'a, T: Clone + 'a> {
    question: &'a str,
    options: Vec<(Cow<'a, str>, T)>,
    suffix: &'a str,
}

pub struct String<'a> {
    question: &'a str,
    default: &'a str,
    initial: std::string::String,
}

pub struct Confirm<'a> {
    question: Cow<'a, str>,
    is_dangerous: bool,
}

pub fn read_choice() -> anyhow::Result<std::string::String> {
    for line in stdin().lock().lines() {
        let line = line.context("reading user input")?;
        return Ok(line.trim().to_lowercase())
    }
    anyhow::bail!("Unexpected end of input");
}

impl<'a, T: Clone + 'a> Numeric<'a, T> {
    pub fn new(question: &'a str) -> Self {
        Numeric {
            question,
            options: Vec::new(),
            suffix: "Your choice?",
        }
    }
    pub fn option<S: Into<Cow<'a, str>>>(&mut self, name: S, value: T)
        -> &mut Self
    {
        self.options.push((name.into(), value));
        self
    }
    pub fn is_empty(&self) -> bool {
        self.options.is_empty()
    }
    //pub fn ask_or(&self, non_interactive: bool, response: ) -> anyhow::Result<T> {
    pub fn ask(&self) -> anyhow::Result<T> {
        let mut editor = Editor::<()>::with_config(Config::builder().build());
        let prompt = format!("{} ", self.suffix);
        loop {
            println!("{}", self.question);
            for (idx, (title, _)) in self.options.iter().enumerate() {
                println!("{}. {}", idx+1, title);
            }
            let value = editor.readline(&prompt)?;
            let choice = match value.parse::<u32>() {
                Ok(choice) => choice,
                Err(e) => {
                    eprintln!("Error reading choice: {}", e);
                    println!("Please enter number");
                    continue;
                }
            };
            if choice == 0 || choice as usize > self.options.len() {
                println!("Please specify a choice from the list above");
                continue;
            }
            return Ok(self.options[(choice-1) as usize].1.clone());
        }
    }
}

impl<'a> String<'a> {
    pub fn new(question: &'a str) -> String {
        String {
            question,
            default: "",
            initial: std::string::String::new(),
        }
    }
    pub fn default(&mut self, default: &'a str) -> &mut Self {
        self.default = default;
        self
    }
    pub fn ask(&mut self) -> anyhow::Result<std::string::String> {
        let prompt = if self.default.is_empty() {
            format!("{}: ", self.question)
        } else {
            format!("{} [{}]: ", self.question, self.default)
        };
        let mut editor = Editor::<()>::with_config(Config::builder().build());
        let mut val = editor.readline_with_initial(
            &prompt,
            (&self.initial, ""),
        )?;
        if val == "" {
            val = self.default.to_string();
        }
        self.initial = val.clone();
        return Ok(val);
    }
}

impl<'a> Confirm<'a> {
    pub fn new<Q: Into<Cow<'a, str>>>(question: Q) -> Confirm<'a> {
        Confirm {
            question: question.into(),
            is_dangerous: false,
        }
    }
    pub fn new_dangerous<Q: Into<Cow<'a, str>>>(question: Q) -> Confirm<'a> {
        Confirm {
            question: question.into(),
            is_dangerous: true,
        }
    }
    pub fn ask(&self) -> anyhow::Result<bool> {
        let mut editor = Editor::<()>::with_config(Config::builder().build());
        let prompt = if self.is_dangerous {
            format!("{} (type `Yes`) ", self.question)
        } else {
            format!("{} [Y/n] ", self.question)
        };
        loop {
            let val = editor.readline(&prompt)?;
            if self.is_dangerous {
                match val.as_ref() {
                    "Yes" => return Ok(true),
                    _ => return Ok(false),
                }
            } else {
                match val.as_ref() {
                    "y" | "Y" | "yes" | "Yes" | "YES" => return Ok(true),
                    "n" | "N" | "no" | "No" | "NO" => return Ok(false),
                    _ => {
                        eprintln!("Please answer Y or N");
                        continue;
                    }
                }
            }
        }
    }
}
