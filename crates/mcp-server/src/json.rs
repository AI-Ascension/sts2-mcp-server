// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;
use std::fmt;

#[path = "json_string.rs"]
mod string;
use string::write_string;
#[path = "json_unicode.rs"]
mod unicode;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(i64),
    String(String),
    Array(Vec<Self>),
    Object(BTreeMap<String, Self>),
}

impl JsonValue {
    pub fn object(entries: impl IntoIterator<Item = (String, Self)>) -> Self {
        Self::Object(entries.into_iter().collect())
    }

    pub fn string(value: impl Into<String>) -> Self {
        Self::String(value.into())
    }

    pub(crate) fn as_object(&self) -> Option<&BTreeMap<String, Self>> {
        match self {
            Self::Object(value) => Some(value),
            _ => None,
        }
    }

    pub(crate) fn as_string(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    pub fn to_json(&self) -> String {
        let mut output = String::new();
        self.write_json(&mut output);
        output
    }

    fn write_json(&self, output: &mut String) {
        match self {
            Self::Null => output.push_str("null"),
            Self::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
            Self::Number(value) => output.push_str(&value.to_string()),
            Self::String(value) => write_string(output, value),
            Self::Array(values) => {
                output.push('[');
                for (index, value) in values.iter().enumerate() {
                    if index > 0 {
                        output.push(',');
                    }
                    value.write_json(output);
                }
                output.push(']');
            }
            Self::Object(values) => {
                output.push('{');
                for (index, (key, value)) in values.iter().enumerate() {
                    if index > 0 {
                        output.push(',');
                    }
                    write_string(output, key);
                    output.push(':');
                    value.write_json(output);
                }
                output.push('}');
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct JsonParseError {
    position: usize,
    message: &'static str,
}

impl JsonParseError {
    fn new(position: usize, message: &'static str) -> Self {
        Self { position, message }
    }
}

impl fmt::Display for JsonParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} at byte {}", self.message, self.position)
    }
}

impl std::error::Error for JsonParseError {}

pub(crate) fn parse(input: &str) -> Result<JsonValue, JsonParseError> {
    let mut parser = Parser {
        bytes: input.as_bytes(),
        position: 0,
        depth: 0,
    };
    let value = parser.value()?;
    parser.whitespace();
    if parser.position != parser.bytes.len() {
        return Err(JsonParseError::new(
            parser.position,
            "unexpected trailing JSON input",
        ));
    }
    Ok(value)
}

/// Parses one bounded JSON value for an owner-local runtime adapter.
pub fn parse_json(input: &str) -> Result<JsonValue, String> {
    parse(input).map_err(|error| error.to_string())
}

struct Parser<'a> {
    bytes: &'a [u8],
    position: usize,
    depth: usize,
}

impl Parser<'_> {
    fn value(&mut self) -> Result<JsonValue, JsonParseError> {
        if self.depth >= 64 {
            return Err(self.error("JSON nesting exceeds the limit"));
        }
        self.depth += 1;
        self.whitespace();
        let result = match self.peek() {
            Some(b'n') => self.literal(b"null", JsonValue::Null),
            Some(b't') => self.literal(b"true", JsonValue::Bool(true)),
            Some(b'f') => self.literal(b"false", JsonValue::Bool(false)),
            Some(b'"') => self.string().map(JsonValue::String),
            Some(b'[') => self.array(),
            Some(b'{') => self.object(),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(_) => Err(self.error("unexpected JSON value")),
            None => Err(self.error("expected JSON value")),
        };
        self.depth -= 1;
        result
    }

    fn literal(&mut self, expected: &[u8], value: JsonValue) -> Result<JsonValue, JsonParseError> {
        if self
            .bytes
            .get(self.position..self.position + expected.len())
            == Some(expected)
        {
            self.position += expected.len();
            Ok(value)
        } else {
            Err(self.error("invalid JSON literal"))
        }
    }

    fn string(&mut self) -> Result<String, JsonParseError> {
        if self.take() != Some(b'"') {
            return Err(self.error("expected JSON string"));
        }
        let mut value = String::new();
        loop {
            match self.take() {
                Some(b'"') => return Ok(value),
                Some(b'\\') => value.push(self.escape()?),
                Some(byte) if byte < 0x20 => return Err(self.error("control byte in JSON string")),
                Some(byte) if byte < 0x80 => value.push(byte as char),
                Some(_) => {
                    let tail = std::str::from_utf8(&self.bytes[self.position - 1..])
                        .map_err(|_| self.error("invalid UTF-8 string"))?;
                    let character = tail
                        .chars()
                        .next()
                        .ok_or_else(|| self.error("invalid string"))?;
                    self.position += character.len_utf8() - 1;
                    value.push(character);
                }
                None => return Err(self.error("unterminated JSON string")),
            }
        }
    }

    fn escape(&mut self) -> Result<char, JsonParseError> {
        match self.take() {
            Some(b'"') => Ok('"'),
            Some(b'\\') => Ok('\\'),
            Some(b'/') => Ok('/'),
            Some(b'b') => Ok('\u{0008}'),
            Some(b'f') => Ok('\u{000c}'),
            Some(b'n') => Ok('\n'),
            Some(b'r') => Ok('\r'),
            Some(b't') => Ok('\t'),
            Some(b'u') => self.unicode_escape(),
            Some(_) => Err(self.error("unsupported JSON escape")),
            None => Err(self.error("unfinished JSON escape")),
        }
    }

    fn number(&mut self) -> Result<JsonValue, JsonParseError> {
        let start = self.position;
        if self.peek() == Some(b'-') {
            self.position += 1;
        }
        let digits = self.position;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.position += 1;
        }
        if self.position == digits {
            return Err(self.error("JSON number has no digits"));
        }
        if self.position - digits > 1 && self.bytes[digits] == b'0' {
            return Err(self.error("leading zero in JSON number"));
        }
        if matches!(self.peek(), Some(b'.' | b'e' | b'E')) {
            return Err(self.error("only integer JSON numbers are supported"));
        }
        let text = std::str::from_utf8(&self.bytes[start..self.position])
            .map_err(|_| self.error("invalid JSON number"))?;
        text.parse::<i64>()
            .map(JsonValue::Number)
            .map_err(|_| self.error("JSON number is out of range"))
    }

    fn array(&mut self) -> Result<JsonValue, JsonParseError> {
        self.take();
        let mut values = Vec::new();
        self.whitespace();
        if self.peek() == Some(b']') {
            self.take();
            return Ok(JsonValue::Array(values));
        }
        loop {
            values.push(self.value()?);
            self.whitespace();
            match self.take() {
                Some(b',') => {}
                Some(b']') => return Ok(JsonValue::Array(values)),
                _ => return Err(self.error("expected comma or closing array bracket")),
            }
        }
    }

    fn object(&mut self) -> Result<JsonValue, JsonParseError> {
        self.take();
        let mut values = BTreeMap::new();
        self.whitespace();
        if self.peek() == Some(b'}') {
            self.take();
            return Ok(JsonValue::Object(values));
        }
        loop {
            self.whitespace();
            let key = self.string()?;
            self.whitespace();
            if self.take() != Some(b':') {
                return Err(self.error("expected object colon"));
            }
            let value = self.value()?;
            if values.insert(key, value).is_some() {
                return Err(self.error("duplicate JSON object key"));
            }
            self.whitespace();
            match self.take() {
                Some(b',') => {}
                Some(b'}') => return Ok(JsonValue::Object(values)),
                _ => return Err(self.error("expected comma or closing object brace")),
            }
        }
    }

    fn whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.position += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.position).copied()
    }

    fn take(&mut self) -> Option<u8> {
        let byte = self.peek()?;
        self.position += 1;
        Some(byte)
    }

    fn error(&self, message: &'static str) -> JsonParseError {
        JsonParseError::new(self.position, message)
    }
}
