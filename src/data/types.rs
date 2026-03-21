use std::ops::Deref;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Selector(pub String);

impl FromStr for Selector {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.trim().is_empty() {
            Err("Selector cannot be empty".to_string())
        } else {
            Ok(Self(s.to_string()))
        }
    }
}
impl Deref for Selector {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct FilePath(pub String);

impl FromStr for FilePath {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.trim().is_empty() {
            Err("Path cannot be empty".to_string())
        } else {
            Ok(Self(s.to_string()))
        }
    }
}
impl Deref for FilePath {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl AsRef<std::path::Path> for FilePath {
    fn as_ref(&self) -> &std::path::Path {
        std::path::Path::new(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct InputValue(pub String);

impl FromStr for InputValue {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.trim().is_empty() {
            Err("Value cannot be empty".to_string())
        } else {
            Ok(Self(s.to_string()))
        }
    }
}
impl Deref for InputValue {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct StorageKey(pub String);

impl FromStr for StorageKey {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.trim().is_empty() {
            return Err("Key cannot be empty".to_string());
        }
        let first_char = s
            .chars()
            .next()
            .ok_or_else(|| "Key cannot be empty".to_string())?;
        if !first_char.is_ascii_alphabetic() && first_char != '_' {
            return Err("Invalid StorageKey".to_string());
        }
        if !s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err("Invalid StorageKey".to_string());
        }
        Ok(Self(s.to_string()))
    }
}
impl Deref for StorageKey {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct JsPayload(pub String);

impl FromStr for JsPayload {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.trim().is_empty() {
            return Err("JS cannot be empty".to_string());
        }
        let dangerous = ["eval(", "Function(", "setTimeout", "setInterval"];
        if dangerous.iter().any(|p| s.contains(p)) {
            return Err("Dangerous Javascript".to_string());
        }
        Ok(Self(s.to_string()))
    }
}
impl Deref for JsPayload {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct CssPayload(pub String);

impl FromStr for CssPayload {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.trim().is_empty() {
            Err("CSS cannot be empty".to_string())
        } else {
            Ok(Self(s.to_string()))
        }
    }
}
impl Deref for CssPayload {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ExpectedText(pub String);

impl FromStr for ExpectedText {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.trim().is_empty() {
            Err("Expected text cannot be empty".to_string())
        } else {
            Ok(Self(s.to_string()))
        }
    }
}
impl Deref for ExpectedText {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Display for Selector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for Selector {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for FilePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for FilePath {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for InputValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for InputValue {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for StorageKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for StorageKey {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for JsPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for JsPayload {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for CssPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for CssPayload {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ExpectedText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for ExpectedText {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
