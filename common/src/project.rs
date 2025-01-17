use once_cell::sync::OnceCell;
use rustrict::{Censor, Type};
use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashSet;
use std::fmt::Formatter;
use std::str::FromStr;

/// Project names must conform to valid Host segments (or labels)
/// as per [IETF RFC 1123](https://datatracker.ietf.org/doc/html/rfc1123).
/// Initially we'll implement a strict subset of the IETF RFC 1123.
/// Additionaly, while host segments are technically case-insensitive, the filesystem isn't,
/// so we restrict project names to be lower case. We also restrict the use of profanity,
/// as well as a list of reserved words.
#[derive(Clone, Serialize, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "backend", derive(sqlx::Type, Hash))]
#[cfg_attr(feature = "backend", sqlx(transparent))]
pub struct ProjectName(String);

impl ProjectName {
    pub fn new(name: &str) -> Result<Self, InvalidProjectName> {
        if Self::is_valid(name) {
            Ok(Self(name.to_owned()))
        } else {
            Err(InvalidProjectName)
        }
    }

    pub fn is_valid(name: &str) -> bool {
        fn is_valid_char(byte: u8) -> bool {
            matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'-')
        }

        fn is_profanity_free(name: &str) -> bool {
            let (_censored, analysis) = Censor::from_str(name).censor_and_analyze();
            !analysis.is(Type::MODERATE_OR_HIGHER)
        }

        fn is_reserved(name: &str) -> bool {
            static INSTANCE: OnceCell<HashSet<&str>> = OnceCell::new();
            INSTANCE.get_or_init(|| {
                HashSet::from(["shuttleapp", "shuttle", "console", "unstable", "staging"])
            });

            INSTANCE
                .get()
                .expect("Reserved words not set")
                .contains(name)
        }

        !name.is_empty()
            && name.len() < 64
            && !name.starts_with('-')
            && !name.ends_with('-')
            && !is_reserved(name)
            && name.bytes().all(is_valid_char)
            && is_profanity_free(name)
    }
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[error(
    "Invalid project name. Project names must:
    1. only contain lowercase alphanumeric characters or dashes `-`.
    2. not start or end with a dash.
    3. not be empty.
    4. be shorter than 64 characters.
    5. not contain any profanities.
    6. not be a reserved word."
)]
pub struct InvalidProjectName;

impl std::ops::Deref for ProjectName {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Display for ProjectName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<'de> Deserialize<'de> for ProjectName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = String::deserialize(deserializer)?;
        s.parse().map_err(DeError::custom)
    }
}

impl FromStr for ProjectName {
    type Err = InvalidProjectName;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ProjectName::new(s)
    }
}

/// Test examples taken from a [Pop-OS project](https://github.com/pop-os/hostname-validator/blob/master/src/lib.rs)
/// and modified to our use case
#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn valid_labels() {
        for name in [
            "50-name",
            "235235",
            "123",
            "kebab-case",
            "lowercase",
            "myassets",
            "dachterrasse",
            "another-valid-project-name",
            "x",
        ] {
            assert!(ProjectName::is_valid(name));
        }
    }

    #[test]
    fn invalid_labels() {
        for name in [
            "UPPERCASE",
            "CamelCase",
            "pascalCase",
            "InVaLid",
            "-invalid-name",
            "also-invalid-",
            "asdf@fasd",
            "@asdfl",
            "asd f@",
            ".invalid",
            "invalid.name",
            "invalid.name.",
            "__dunder_like__",
            "__invalid",
            "invalid__",
            "test-condom-condom",
            "s________e",
            "snake_case",
            "exactly-16-chars\
            exactly-16-chars\
            exactly-16-chars\
            exactly-16-chars",
            "shuttle",
            "shuttleapp",
            "",
        ] {
            assert!(!ProjectName::is_valid(name));
        }
    }
}
