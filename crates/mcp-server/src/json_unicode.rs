// SPDX-License-Identifier: MIT

use super::{JsonParseError, Parser};

impl Parser<'_> {
    pub(super) fn unicode_escape(&mut self) -> Result<char, JsonParseError> {
        let first = self.hex_quad()?;
        let value = if (0xd800..=0xdbff).contains(&first) {
            if self.take() != Some(b'\\') || self.take() != Some(b'u') {
                return Err(self.error("missing low surrogate"));
            }
            let second = self.hex_quad()?;
            if !(0xdc00..=0xdfff).contains(&second) {
                return Err(self.error("invalid low surrogate"));
            }
            0x10000 + ((first - 0xd800) << 10) + second - 0xdc00
        } else {
            first
        };
        char::from_u32(value).ok_or_else(|| self.error("invalid unicode scalar"))
    }

    fn hex_quad(&mut self) -> Result<u32, JsonParseError> {
        let mut value = 0_u32;
        for _ in 0..4 {
            let byte = self
                .take()
                .ok_or_else(|| self.error("unfinished unicode escape"))?;
            value = value * 16
                + match byte {
                    b'0'..=b'9' => u32::from(byte - b'0'),
                    b'a'..=b'f' => u32::from(byte - b'a' + 10),
                    b'A'..=b'F' => u32::from(byte - b'A' + 10),
                    _ => return Err(self.error("invalid unicode escape")),
                };
        }
        Ok(value)
    }
}
