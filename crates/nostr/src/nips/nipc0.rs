// Copyright (c) 2022-2023 Yuki Kishimoto
// Copyright (c) 2023-2025 Rust Nostr Developers
// Distributed under the MIT software license

//! NIPC0: Code Snippets
//!
//! <https://github.com/nostr-protocol/nips/blob/master/C0.md>

use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use core::fmt;

use crate::event::tag::{Tag, TagCodec, impl_tag_codec_conversions};
use crate::{EventBuilder, Kind};

const LANGUAGE: &str = "l";
const NAME: &str = "name";
const EXTENSION: &str = "extension";
const DESCRIPTION: &str = "description";
const RUNTIME: &str = "runtime";
const LICENSE: &str = "license";
const DEPENDENCY: &str = "dep";
const REPOSITORY: &str = "repo";

/// NIP-C0 error
#[derive(Debug, PartialEq)]
pub enum Error {
    /// Missing dependency
    MissingDependency,
    /// Missing description
    MissingDescription,
    /// Missing extension
    MissingExtension,
    /// Missing language
    MissingLanguage,
    /// Missing license
    MissingLicense,
    /// Missing name
    MissingName,
    /// Missing repository
    MissingRepository,
    /// Missing runtime
    MissingRuntime,
    /// Missing tag kind
    MissingTagKind,
    /// Unknown tag
    UnknownTag,
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingDependency => f.write_str("Missing dependency"),
            Self::MissingDescription => f.write_str("Missing description"),
            Self::MissingExtension => f.write_str("Missing extension"),
            Self::MissingLanguage => f.write_str("Missing language"),
            Self::MissingLicense => f.write_str("Missing license"),
            Self::MissingName => f.write_str("Missing name"),
            Self::MissingRepository => f.write_str("Missing repository"),
            Self::MissingRuntime => f.write_str("Missing runtime"),
            Self::MissingTagKind => f.write_str("Missing tag kind"),
            Self::UnknownTag => f.write_str("Unknown tag"),
        }
    }
}

/// Standardized NIP-C0 tags
///
/// <https://github.com/nostr-protocol/nips/blob/master/C0.md>
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NipC0Tag {
    /// `l` tag used as programming language
    Language(String),
    /// `name` tag
    Name(String),
    /// `extension` tag
    Extension(String),
    /// `description` tag
    Description(String),
    /// `runtime` tag
    Runtime(String),
    /// `license` tag
    License(String),
    /// `dep` tag
    Dependency(String),
    /// `repo` tag
    Repository(String),
}

impl TagCodec for NipC0Tag {
    type Error = Error;

    fn parse<I, S>(tag: I) -> Result<Self, Self::Error>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut iter = tag.into_iter();

        let kind: S = iter.next().ok_or(Error::MissingTagKind)?;

        match kind.as_ref() {
            LANGUAGE => Ok(Self::Language(parse_language_tag(iter)?)),
            NAME => Ok(Self::Name(parse_name_tag(iter)?)),
            EXTENSION => Ok(Self::Extension(parse_extension_tag(iter)?)),
            DESCRIPTION => Ok(Self::Description(parse_description_tag(iter)?)),
            RUNTIME => Ok(Self::Runtime(parse_runtime_tag(iter)?)),
            LICENSE => Ok(Self::License(parse_license_tag(iter)?)),
            DEPENDENCY => Ok(Self::Dependency(parse_dependency_tag(iter)?)),
            REPOSITORY => Ok(Self::Repository(parse_repository_tag(iter)?)),
            _ => Err(Error::UnknownTag),
        }
    }

    fn to_tag(&self) -> Tag {
        match self {
            Self::Language(language) => {
                Tag::new(vec![String::from(LANGUAGE), language.to_lowercase()])
            }
            Self::Name(name) => Tag::new(vec![String::from(NAME), name.clone()]),
            Self::Extension(extension) => {
                Tag::new(vec![String::from(EXTENSION), extension.clone()])
            }
            Self::Description(description) => {
                Tag::new(vec![String::from(DESCRIPTION), description.clone()])
            }
            Self::Runtime(runtime) => Tag::new(vec![String::from(RUNTIME), runtime.clone()]),
            Self::License(license) => Tag::new(vec![String::from(LICENSE), license.clone()]),
            Self::Dependency(dependency) => {
                Tag::new(vec![String::from(DEPENDENCY), dependency.clone()])
            }
            Self::Repository(repository) => {
                Tag::new(vec![String::from(REPOSITORY), repository.clone()])
            }
        }
    }
}

impl_tag_codec_conversions!(NipC0Tag);

fn parse_language_tag<T, S>(mut iter: T) -> Result<String, Error>
where
    T: Iterator<Item = S>,
    S: AsRef<str>,
{
    let language: S = iter.next().ok_or(Error::MissingLanguage)?;
    Ok(language.as_ref().to_lowercase())
}

fn parse_name_tag<T, S>(mut iter: T) -> Result<String, Error>
where
    T: Iterator<Item = S>,
    S: AsRef<str>,
{
    let name: S = iter.next().ok_or(Error::MissingName)?;
    Ok(name.as_ref().to_string())
}

fn parse_extension_tag<T, S>(mut iter: T) -> Result<String, Error>
where
    T: Iterator<Item = S>,
    S: AsRef<str>,
{
    let extension: S = iter.next().ok_or(Error::MissingExtension)?;
    Ok(extension.as_ref().to_string())
}

fn parse_description_tag<T, S>(mut iter: T) -> Result<String, Error>
where
    T: Iterator<Item = S>,
    S: AsRef<str>,
{
    let description: S = iter.next().ok_or(Error::MissingDescription)?;
    Ok(description.as_ref().to_string())
}

fn parse_runtime_tag<T, S>(mut iter: T) -> Result<String, Error>
where
    T: Iterator<Item = S>,
    S: AsRef<str>,
{
    let runtime: S = iter.next().ok_or(Error::MissingRuntime)?;
    Ok(runtime.as_ref().to_string())
}

fn parse_license_tag<T, S>(mut iter: T) -> Result<String, Error>
where
    T: Iterator<Item = S>,
    S: AsRef<str>,
{
    let license: S = iter.next().ok_or(Error::MissingLicense)?;
    Ok(license.as_ref().to_string())
}

fn parse_dependency_tag<T, S>(mut iter: T) -> Result<String, Error>
where
    T: Iterator<Item = S>,
    S: AsRef<str>,
{
    let dependency: S = iter.next().ok_or(Error::MissingDependency)?;
    Ok(dependency.as_ref().to_string())
}

fn parse_repository_tag<T, S>(mut iter: T) -> Result<String, Error>
where
    T: Iterator<Item = S>,
    S: AsRef<str>,
{
    let repository: S = iter.next().ok_or(Error::MissingRepository)?;
    Ok(repository.as_ref().to_string())
}

/// Code snippet
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CodeSnippet {
    /// The code snippet.
    pub snippet: String,
    /// Programming language name.
    /// Examples: "javascript", "python", "rust"
    pub language: Option<String>,
    /// Name of the code snippet, commonly a filename.
    /// Examples: "hello-world.js", "quick-sort.py"
    pub name: Option<String>,
    /// File extension (without the dot).
    /// Examples: "js", "py", "rs"
    pub extension: Option<String>,
    /// Brief description of what the code does
    pub description: Option<String>,
    /// Runtime or environment specification.
    /// Example: "node v18.15.0", "python 3.11"
    pub runtime: Option<String>,
    /// License under which the code is shared.
    /// Example: "MIT", "GPL-3.0", "Apache-2.0"
    pub license: Option<String>,
    /// Dependencies required for the code to run.
    pub dependencies: Vec<String>,
    /// Reference to a repository where this code originates.
    pub repo: Option<String>,
}

impl CodeSnippet {
    /// Create a new code snippet
    #[inline]
    pub fn new<T>(snippet: T) -> Self
    where
        T: Into<String>,
    {
        Self {
            snippet: snippet.into(),
            ..Default::default()
        }
    }

    /// Set the programming language name (e.g. "javascript", "python", "rust").
    #[inline]
    pub fn language<T>(mut self, lang: T) -> Self
    where
        T: AsRef<str>,
    {
        self.language = Some(lang.as_ref().to_lowercase());
        self
    }

    /// Set the name of the code snippet, commonly a filename.
    #[inline]
    pub fn name<T>(mut self, name: T) -> Self
    where
        T: Into<String>,
    {
        self.name = Some(name.into());
        self
    }

    /// Set the file extension (without the dot).
    #[inline]
    pub fn extension<T>(mut self, extension: T) -> Self
    where
        T: Into<String>,
    {
        self.extension = Some(extension.into());
        self
    }

    /// Set a brief description of what the code does
    #[inline]
    pub fn description<T>(mut self, description: T) -> Self
    where
        T: Into<String>,
    {
        self.description = Some(description.into());
        self
    }

    /// Set the runtime or environment specification (e.g. "node v18.15.0", "python 3.11").
    #[inline]
    pub fn runtime<T>(mut self, runtime: T) -> Self
    where
        T: Into<String>,
    {
        self.runtime = Some(runtime.into());
        self
    }

    /// Set the license under which the code is shared (e.g. "MIT", "GPL-3.0", "Apache-2.0").
    #[inline]
    pub fn license<T>(mut self, license: T) -> Self
    where
        T: Into<String>,
    {
        self.license = Some(license.into());
        self
    }

    /// Add a dependency required for the code to run.
    pub fn dependencies<T>(mut self, dep: T) -> Self
    where
        T: Into<String>,
    {
        let dep = dep.into();
        if !self.dependencies.contains(&dep) {
            self.dependencies.push(dep);
        }
        self
    }

    /// Set the repository where this code originates.
    #[inline]
    pub fn repo<T>(mut self, repo: T) -> Self
    where
        T: Into<String>,
    {
        self.repo = Some(repo.into());
        self
    }

    /// Convert the code snippet to an event builder
    #[allow(clippy::wrong_self_convention)]
    pub(crate) fn to_event_builder(self) -> EventBuilder {
        let mut tags: Vec<Tag> = Vec::new();

        let mut add_if_some = |tag: Option<NipC0Tag>| {
            if let Some(tag) = tag {
                tags.push(tag.into());
            }
        };

        add_if_some(self.language.map(NipC0Tag::Language));
        add_if_some(self.name.map(NipC0Tag::Name));
        add_if_some(self.extension.map(NipC0Tag::Extension));
        add_if_some(self.description.map(NipC0Tag::Description));
        add_if_some(self.runtime.map(NipC0Tag::Runtime));
        add_if_some(self.license.map(NipC0Tag::License));
        add_if_some(self.repo.map(NipC0Tag::Repository));

        for dep in self.dependencies.into_iter() {
            tags.push(NipC0Tag::Dependency(dep).into());
        }

        EventBuilder::new(Kind::CodeSnippet, self.snippet).tags(tags)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_language_tag() {
        let tag = vec!["l", "Rust"];
        let parsed = NipC0Tag::parse(&tag).unwrap();
        assert_eq!(parsed, NipC0Tag::Language(String::from("rust")));
        assert_eq!(parsed.to_tag(), Tag::parse(vec!["l", "rust"]).unwrap());
    }

    #[test]
    fn test_parse_name_tag() {
        let tag = vec!["name", "hello-world.rs"];
        let parsed = NipC0Tag::parse(&tag).unwrap();
        assert_eq!(parsed, NipC0Tag::Name(String::from("hello-world.rs")));
        assert_eq!(parsed.to_tag(), Tag::parse(tag).unwrap());
    }

    #[test]
    fn test_parse_extension_tag() {
        let tag = vec!["extension", "rs"];
        let parsed = NipC0Tag::parse(&tag).unwrap();
        assert_eq!(parsed, NipC0Tag::Extension(String::from("rs")));
        assert_eq!(parsed.to_tag(), Tag::parse(tag).unwrap());
    }

    #[test]
    fn test_parse_description_tag() {
        let tag = vec!["description", "Prints Hello, Nostr!"];
        let parsed = NipC0Tag::parse(&tag).unwrap();
        assert_eq!(
            parsed,
            NipC0Tag::Description(String::from("Prints Hello, Nostr!"))
        );
        assert_eq!(parsed.to_tag(), Tag::parse(tag).unwrap());
    }

    #[test]
    fn test_parse_runtime_tag() {
        let tag = vec!["runtime", "rustc 1.70.0"];
        let parsed = NipC0Tag::parse(&tag).unwrap();
        assert_eq!(parsed, NipC0Tag::Runtime(String::from("rustc 1.70.0")));
        assert_eq!(parsed.to_tag(), Tag::parse(tag).unwrap());
    }

    #[test]
    fn test_parse_license_tag() {
        let tag = vec!["license", "MIT"];
        let parsed = NipC0Tag::parse(&tag).unwrap();
        assert_eq!(parsed, NipC0Tag::License(String::from("MIT")));
        assert_eq!(parsed.to_tag(), Tag::parse(tag).unwrap());
    }

    #[test]
    fn test_parse_dependency_tag() {
        let tag = vec!["dep", "serde"];
        let parsed = NipC0Tag::parse(&tag).unwrap();
        assert_eq!(parsed, NipC0Tag::Dependency(String::from("serde")));
        assert_eq!(parsed.to_tag(), Tag::parse(tag).unwrap());
    }

    #[test]
    fn test_parse_repository_tag() {
        let tag = vec!["repo", "https://github.com/nostr-protocol/nostr"];
        let parsed = NipC0Tag::parse(&tag).unwrap();
        assert_eq!(
            parsed,
            NipC0Tag::Repository(String::from("https://github.com/nostr-protocol/nostr"))
        );
        assert_eq!(parsed.to_tag(), Tag::parse(tag).unwrap());
    }

    #[test]
    fn test_code_snippet_to_event_builder() {
        let snippet = CodeSnippet::new("fn main() {}")
            .language("Rust")
            .name("main.rs")
            .extension("rs")
            .description("A minimal Rust program")
            .runtime("rustc 1.70.0")
            .license("MIT")
            .dependencies("serde")
            .repo("https://github.com/nostr-protocol/nostr");

        let builder = snippet.to_event_builder();
        let expected = EventBuilder::new(Kind::CodeSnippet, "fn main() {}").tags([
            NipC0Tag::Language(String::from("rust")).to_tag(),
            NipC0Tag::Name(String::from("main.rs")).to_tag(),
            NipC0Tag::Extension(String::from("rs")).to_tag(),
            NipC0Tag::Description(String::from("A minimal Rust program")).to_tag(),
            NipC0Tag::Runtime(String::from("rustc 1.70.0")).to_tag(),
            NipC0Tag::License(String::from("MIT")).to_tag(),
            NipC0Tag::Repository(String::from("https://github.com/nostr-protocol/nostr")).to_tag(),
            NipC0Tag::Dependency(String::from("serde")).to_tag(),
        ]);

        assert_eq!(builder, expected);
    }
}
