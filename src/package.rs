use std::{
    collections::HashMap, env::current_dir, fmt::Display, process::Command, str::FromStr,
    sync::LazyLock,
};

use anyhow::{anyhow, Result};
use camino::Utf8PathBuf;
use regex::{Captures, Regex};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use spdx::Expression;

#[derive(Debug, Serialize, Deserialize)]
pub struct Package {
    pub info: Info,
    pub dependencies: Option<Dependencies>,
    #[serde(default)]
    pub sources: Vec<Source>,
    #[serde(default)]
    pub steps: Vec<Step>,
    #[serde(default)]
    pub directories: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Dependencies {
    #[serde(default)]
    pub required: Vec<String>,
    #[serde(default)]
    pub optional: Vec<String>,
    #[serde(default)]
    pub build: Vec<String>,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct Info {
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde_as(as = "DisplayFromStr")]
    pub license: Expression,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Source {
    pub url: String,
    pub checksum: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Step {
    pub name: String,
    #[serde(flatten)]
    pub variant: StepVariant,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StepVariant {
    Command {
        #[serde_as(as = "DisplayFromStr")]
        runner: Runner,
        command: String,
    },
    Move {
        path: Utf8PathBuf,
    },
}

#[derive(Debug)]
pub enum Runner {
    Shell,
}

impl Display for Runner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Shell => write!(f, "shell"),
        }
    }
}

impl FromStr for Runner {
    // FIXME: use an actual error
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "shell" => Ok(Self::Shell),
            _ => Err(anyhow!("Unknown runner")),
        }
    }
}

impl Runner {
    pub fn into_command(&self) -> Command {
        match self {
            Self::Shell => {
                let mut command = Command::new("/bin/sh");

                command.arg("-c");

                command
            }
        }
    }
}

static VARIABLE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"%\{([^}]+)\}").expect("invalid regex"));

impl Package {
    pub fn parse(s: &str) -> Result<Self> {
        let mut package: Package = toml_edit::de::from_str(s)?;

        let mut variables = HashMap::new();

        let pkgdir = current_dir()?.join("package");
        variables.insert("version", package.info.version.as_str());
        variables.insert("pkgdir", pkgdir.to_str().unwrap()); // FIXME: horrible

        for source in package.sources.iter_mut() {
            source.url = replace_vars(&source.url, &variables)
        }

        for step in package.steps.iter_mut() {
            match &mut step.variant {
                StepVariant::Command { command, .. } => {
                    *command = replace_vars(command.as_str(), &variables).into();
                }
                StepVariant::Move { path } => {
                    *path = replace_vars(path.as_str(), &variables).into();
                }
            }
        }

        Ok(package)
    }
}

fn replace_vars<'h>(haystack: &'h str, variables: &HashMap<&str, &str>) -> String {
    VARIABLE_REGEX
        .replace_all(haystack, |caps: &Captures| {
            variables.get(&caps[1]).expect("Unknown variable") // FIXME: error handling
        })
        .into_owned()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn test_basic_replacement() {
        let mut variables = HashMap::new();
        variables.insert("name", "Mati");
        variables.insert("greeting", "Hello");

        let haystack = "%{greeting}, %{name}!";
        assert_eq!(replace_vars(haystack, &variables), "Hello, Mati!");
    }

    #[test]
    #[should_panic]
    fn test_missing_variable() {
        let variables = HashMap::new();
        let haystack = "Hi %{name}!";

        replace_vars(haystack, &variables);
    }

    #[test]
    fn test_multiple_occurrences() {
        let mut variables = HashMap::new();
        variables.insert("word", "rust");

        let haystack = "I love %{word}! %{word} is great!";
        assert_eq!(
            replace_vars(haystack, &variables),
            "I love rust! rust is great!"
        );
    }

    #[test]
    #[should_panic]
    fn test_multiple_missing_variables() {
        let variables = HashMap::new();
        let haystack = "Hi %{name}, welcome to %{city}!";

        replace_vars(haystack, &variables);
    }
}
