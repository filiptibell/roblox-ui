use std::fmt;
use std::num::{ParseFloatError, ParseIntError};
use std::str::{FromStr, ParseBoolError};

use anyhow::Result;
use serde::Serialize;
use serde_json::Value as JsonValue;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum ValueKindParseError {
    #[error("unknown event value kind '{0}'")]
    Unknown(String),
}

#[derive(Debug, Clone, Copy)]
pub enum ValueKind {
    Bool,
    Double,
    Integer,
    String,
    Token,
}

impl fmt::Display for ValueKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Bool => "Bool",
            Self::Double => "Double",
            Self::Integer => "Integer",
            Self::String => "String",
            Self::Token => "Token",
        };
        s.fmt(f)
    }
}

impl FromStr for ValueKind {
    type Err = ValueKindParseError;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_ref() {
            "bool" => Ok(Self::Bool),
            "float" | "f32" | "f64" | "double" => Ok(Self::Double),
            "int" | "int32" | "int64" | "integer" => Ok(Self::Integer),
            "string" => Ok(Self::String),
            "token" => Ok(Self::Token),
            s => Err(ValueKindParseError::Unknown(s.to_string())),
        }
    }
}

#[derive(Debug, Clone, Error)]
pub enum ValueParseError {
    #[error("invalid conversion-  {0}")]
    InvalidConversion(&'static str),
    #[error(transparent)]
    InvalidBool(#[from] ParseBoolError),
    #[error(transparent)]
    ParseFloat(#[from] ParseFloatError),
    #[error(transparent)]
    ParseInt(#[from] ParseIntError),
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Value {
    None,
    Bool(bool),
    Double(f64),
    Integer(i64),
    String(String),
    Token(String),
}

impl Value {
    pub fn as_string(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            Self::Token(s) => Some(s),
            _ => None,
        }
    }

    pub fn parse(kind: ValueKind, name: &str, s: &str) -> Result<Self, ValueParseError> {
        // NOTE: FFlag values have some special handling internally at roblox ?
        // Their values are marked as bool/double/etc but the value is a string
        // which is just the FFlag name, so we just dont bother parsing FFlags
        if name.eq_ignore_ascii_case("fflag") {
            return Ok(Self::None);
        }

        match kind {
            ValueKind::Bool => {
                let trimmed = s.trim().to_ascii_lowercase();
                match trimmed.as_ref() {
                    "0" => Ok(Self::Bool(false)),
                    "1" => Ok(Self::Bool(true)),
                    _ => Ok(Self::Bool(trimmed.parse::<bool>()?)),
                }
            }
            ValueKind::Double => {
                let trimmed = s.trim().to_ascii_lowercase();
                Ok(Self::Double(trimmed.parse::<f64>()?))
            }
            ValueKind::Integer => {
                let trimmed = s.trim().to_ascii_lowercase();
                Ok(Self::Integer(trimmed.parse::<i64>()?))
            }
            ValueKind::String => Ok(Self::String(s.to_string())),
            ValueKind::Token => Ok(Self::Token(s.to_string())),
        }
    }

    pub fn coerce(&self, target: ValueKind) -> Result<Self, ValueParseError> {
        match self {
            Self::None => Err(ValueParseError::InvalidConversion(
                "None can not be converted",
            )),
            Self::Bool(b) => match target {
                ValueKind::Bool => Ok(Self::Bool(*b)),
                ValueKind::Double => Err(ValueParseError::InvalidConversion(
                    "Bool can not be converted into Double",
                )),
                ValueKind::Integer => Err(ValueParseError::InvalidConversion(
                    "Bool can not be converted into Integer",
                )),
                ValueKind::String => Ok(Self::String(b.to_string())),
                ValueKind::Token => Ok(Self::Token(b.to_string())),
            },
            Self::Double(n) => match target {
                ValueKind::Bool => Err(ValueParseError::InvalidConversion(
                    "Double can not be converted into Bool",
                )),
                ValueKind::Double => Ok(Self::Double(*n)),
                ValueKind::Integer => {
                    Self::parse(target, "<<number-conversion>>", n.to_string().as_ref())
                }
                ValueKind::String => Ok(Self::String(n.to_string())),
                ValueKind::Token => Ok(Self::Token(n.to_string())),
            },
            Self::Integer(i) => match target {
                ValueKind::Bool => Err(ValueParseError::InvalidConversion(
                    "Integer can not be converted into Bool",
                )),
                ValueKind::Double => {
                    Self::parse(target, "<<number-conversion>>", i.to_string().as_ref())
                }
                ValueKind::Integer => Ok(Self::Integer(*i)),
                ValueKind::String => Ok(Self::String(i.to_string())),
                ValueKind::Token => Ok(Self::Token(i.to_string())),
            },
            Self::String(s) => Self::parse(target, "<<string-conversion>>", s),
            Self::Token(s) => Self::parse(target, "<<string-conversion>>", s),
        }
    }
}

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let json_value = match self {
            Self::None => JsonValue::Null,
            Self::Bool(b) => JsonValue::Bool(*b),
            Self::Double(n) => JsonValue::Number(
                serde_json::Number::from_f64(*n)
                    .expect("nan and inf values are not serializable as json"),
            ),
            Self::Integer(i) => JsonValue::Number(serde_json::Number::from(*i)),
            Self::String(s) => JsonValue::String(s.clone()),
            Self::Token(s) => JsonValue::String(s.clone()),
        };
        json_value.serialize(serializer)
    }
}
