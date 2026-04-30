// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use crate::ReadExt;
use anyhow::{anyhow, bail};
use kaloron::Version;

pub struct TextReader<R: ReadExt> {
    version: Version,
    input: R,
    peeked: Option<u8>,
    pos: usize,
}

#[derive(Clone, Copy)]
enum AtomReadMode {
    Whole,
    Prefix,
    Continuation,
}

impl<R: ReadExt> TextReader<R> {
    pub fn new(version: Version, input: R) -> Self {
        Self {
            version,
            input,
            peeked: None,
            pos: 0,
        }
    }

    pub fn finish(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn read_raw_byte(&mut self) -> anyhow::Result<Option<u8>> {
        let mut byte = [0u8; 1];
        match self.input.read(&mut byte)? {
            0 => Ok(None),
            1 => Ok(Some(byte[0])),
            _ => unreachable!(),
        }
    }

    fn peek_byte(&mut self) -> anyhow::Result<Option<u8>> {
        if self.peeked.is_none() {
            self.peeked = self.read_raw_byte()?;
        }
        Ok(self.peeked)
    }

    fn next_byte(&mut self) -> anyhow::Result<Option<u8>> {
        let byte = self.peek_byte()?;
        if byte.is_some() {
            self.peeked = None;
            self.pos += 1;
        }
        Ok(byte)
    }

    pub fn position(&self) -> usize {
        self.pos
    }

    pub fn version(&self) -> Version {
        self.version
    }

    fn expect_byte(&mut self, expected: u8) -> anyhow::Result<()> {
        match self.next_byte()? {
            Some(found) if found == expected => Ok(()),
            Some(found) => bail!(
                "expected byte '{}' at position {}, found '{}'",
                expected as char,
                self.pos - 1,
                found as char
            ),
            None => bail!("expected byte '{}' at end of input", expected as char),
        }
    }

    fn take_atom_text(&mut self) -> anyhow::Result<String> {
        let start = self.pos;
        let mut out = Vec::new();
        while let Some(byte) = self.peek_byte()? {
            if is_atom_terminator(byte) {
                break;
            }
            out.push(self.next_byte()?.expect("peeked byte disappeared"));
        }
        if out.is_empty() {
            bail!("expected atom at byte {}", start);
        }
        Ok(String::from_utf8(out)?)
    }

    fn take_exact_atom(&mut self, expected: &str, mode: AtomReadMode) -> anyhow::Result<()> {
        for expected_byte in expected.bytes() {
            match self.next_byte()? {
                Some(found) if found == expected_byte => {}
                Some(found) => bail!(
                    "expected byte '{}' at position {}, found '{}'",
                    expected_byte as char,
                    self.pos - 1,
                    found as char
                ),
                None => bail!("expected atom {expected:?} at end of input"),
            }
        }

        if matches!(mode, AtomReadMode::Whole) {
            match self.peek_byte()? {
                Some(byte) if !is_atom_terminator(byte) => bail!("expected atom {expected:?}"),
                _ => {}
            }
        }

        Ok(())
    }

    pub fn take_none(&mut self) -> anyhow::Result<()> {
        self.take_exact_atom("none", AtomReadMode::Whole)
    }

    pub fn maybe_take_none(&mut self) -> anyhow::Result<bool> {
        match self.peek_byte()? {
            Some(b'n') => {
                self.take_none()?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    pub fn take_unit(&mut self) -> anyhow::Result<()> {
        self.take_exact_atom("unit", AtomReadMode::Whole)
    }

    fn read_utf8_char(&mut self, first: u8) -> anyhow::Result<char> {
        if first < 0x80 {
            return Ok(first as char);
        }

        let width = match first {
            0xC2..=0xDF => 2,
            0xE0..=0xEF => 3,
            0xF0..=0xF4 => 4,
            _ => bail!("invalid utf-8 in string literal"),
        };

        let mut buf = [0u8; 4];
        buf[0] = first;
        for slot in buf[1..width].iter_mut() {
            let byte = self
                .next_byte()?
                .ok_or_else(|| anyhow!("invalid utf-8 in string literal"))?;
            if (byte & 0b1100_0000) != 0b1000_0000 {
                bail!("invalid utf-8 in string literal");
            }
            *slot = byte;
        }

        let text = std::str::from_utf8(&buf[..width])
            .map_err(|_| anyhow!("invalid utf-8 in string literal"))?;
        Ok(text.chars().next().unwrap())
    }

    fn parse_string_into(&mut self, mut out: Option<&mut String>) -> anyhow::Result<()> {
        self.expect_byte(b'"')?;

        loop {
            let byte = self
                .next_byte()?
                .ok_or_else(|| anyhow!("unexpected end of input while parsing string"))?;
            match byte {
                b'"' => return Ok(()),
                b'\\' => {
                    let escaped = self
                        .next_byte()?
                        .ok_or_else(|| anyhow!("unexpected end of input after escape"))?;
                    let ch = match escaped {
                        b'"' => Some('"'),
                        b'\\' => Some('\\'),
                        b'/' => Some('/'),
                        b'b' => Some('\u{0008}'),
                        b'f' => Some('\u{000C}'),
                        b'n' => Some('\n'),
                        b'r' => Some('\r'),
                        b't' => Some('\t'),
                        b'u' => {
                            let first = self.read_hex4()?;
                            let ch = if (0xD800..=0xDBFF).contains(&first) {
                                self.expect_byte(b'\\')?;
                                self.expect_byte(b'u')?;
                                let second = self.read_hex4()?;
                                if !(0xDC00..=0xDFFF).contains(&second) {
                                    bail!("invalid low surrogate in unicode escape");
                                }
                                let code_point = 0x10000
                                    + (((first as u32 - 0xD800) << 10) | (second as u32 - 0xDC00));
                                char::from_u32(code_point)
                                    .ok_or_else(|| anyhow!("invalid unicode escape"))?
                            } else if (0xDC00..=0xDFFF).contains(&first) {
                                bail!("unexpected low surrogate in unicode escape");
                            } else {
                                char::from_u32(first as u32)
                                    .ok_or_else(|| anyhow!("invalid unicode escape"))?
                            };
                            Some(ch)
                        }
                        other => bail!("invalid escape sequence '\\{}'", other as char),
                    };
                    if let Some(ch) = ch
                        && let Some(out) = out.as_deref_mut()
                    {
                        out.push(ch);
                    }
                }
                control if control < 0x20 => bail!("control character in string literal"),
                first => {
                    let ch = self.read_utf8_char(first)?;
                    if let Some(out) = out.as_deref_mut() {
                        out.push(ch);
                    }
                }
            }
        }
    }

    pub fn take_string(&mut self) -> anyhow::Result<String> {
        let mut out = String::new();
        self.parse_string_into(Some(&mut out))?;
        Ok(out)
    }

    fn skip_string(&mut self) -> anyhow::Result<()> {
        self.parse_string_into(None)
    }

    pub fn take_char(&mut self) -> anyhow::Result<char> {
        let text = self.take_string()?;
        let mut chars = text.chars();
        let ch = chars
            .next()
            .ok_or_else(|| anyhow!("expected a string of length 1 for char"))?;
        if chars.next().is_some() {
            bail!("expected a string of length 1 for char");
        }
        Ok(ch)
    }

    fn read_hex4(&mut self) -> anyhow::Result<u16> {
        let mut buf = [0u8; 4];
        for byte in &mut buf {
            *byte = self
                .next_byte()?
                .ok_or_else(|| anyhow!("unexpected end of input in unicode escape"))?;
        }
        let hex = std::str::from_utf8(&buf).map_err(|_| anyhow!("invalid unicode escape"))?;
        if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            bail!("invalid unicode escape");
        }
        Ok(u16::from_str_radix(hex, 16)?)
    }

    pub fn take_bool(&mut self) -> anyhow::Result<bool> {
        match self.take_atom_text()?.as_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            other => bail!("expected bool, got {other:?}"),
        }
    }

    pub fn take_u8(&mut self) -> anyhow::Result<u8> {
        let value = self.take_unsigned_value("u8:", "u8")?;
        u8::try_from(value).map_err(|_| anyhow!("value is out of range for u8"))
    }

    pub fn take_u16(&mut self) -> anyhow::Result<u16> {
        let value = self.take_unsigned_value("u16:", "u16")?;
        u16::try_from(value).map_err(|_| anyhow!("value is out of range for u16"))
    }

    pub fn take_u32(&mut self) -> anyhow::Result<u32> {
        let value = self.take_unsigned_value("u32:", "u32")?;
        u32::try_from(value).map_err(|_| anyhow!("value is out of range for u32"))
    }

    pub fn take_u64(&mut self) -> anyhow::Result<u64> {
        let value = self.take_unsigned_value("u64:", "u64")?;
        u64::try_from(value).map_err(|_| anyhow!("value is out of range for u64"))
    }

    pub fn take_u128(&mut self) -> anyhow::Result<u128> {
        self.take_unsigned_value("u128:", "u128")
    }

    pub fn take_i8(&mut self) -> anyhow::Result<i8> {
        let (negative, magnitude) = self.take_signed_value("i8")?;
        convert_signed_primitive(negative, magnitude, i8::MAX as u128, (i8::MAX as u128) + 1)
    }

    pub fn take_i16(&mut self) -> anyhow::Result<i16> {
        let (negative, magnitude) = self.take_signed_value("i16")?;
        convert_signed_primitive(
            negative,
            magnitude,
            i16::MAX as u128,
            (i16::MAX as u128) + 1,
        )
    }

    pub fn take_i32(&mut self) -> anyhow::Result<i32> {
        let (negative, magnitude) = self.take_signed_value("i32")?;
        convert_signed_primitive(
            negative,
            magnitude,
            i32::MAX as u128,
            (i32::MAX as u128) + 1,
        )
    }

    pub fn take_i64(&mut self) -> anyhow::Result<i64> {
        let (negative, magnitude) = self.take_signed_value("i64")?;
        convert_signed_primitive(
            negative,
            magnitude,
            i64::MAX as u128,
            (i64::MAX as u128) + 1,
        )
    }

    pub fn take_i128(&mut self) -> anyhow::Result<i128> {
        let (negative, magnitude) = self.take_signed_value("i128")?;
        convert_signed_primitive(
            negative,
            magnitude,
            i128::MAX as u128,
            (i128::MAX as u128) + 1,
        )
    }

    pub fn take_f32(&mut self) -> anyhow::Result<f32> {
        self.take_exact_atom("f32:", AtomReadMode::Prefix)?;
        Ok(f32::from_bits(
            self.take_fixed_width_hex_bits("f32", 8)? as u32
        ))
    }

    pub fn take_f64(&mut self) -> anyhow::Result<f64> {
        self.take_exact_atom("f64:", AtomReadMode::Prefix)?;
        Ok(f64::from_bits(self.take_fixed_width_hex_bits("f64", 16)?))
    }

    pub fn start_array(&mut self) -> anyhow::Result<Option<usize>> {
        self.expect_byte(b'[')?;

        match self.peek_byte()? {
            Some(b'$') => {
                self.next_byte()?;
                Ok(None)
            }
            Some(b']') => bail!("array length marker missing at byte {}", self.pos),
            Some(_) => {
                let mut digits = Vec::new();
                while let Some(byte) = self.peek_byte()? {
                    if byte == b'$' {
                        break;
                    }
                    if !(byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)) {
                        bail!("invalid array length marker at byte {}", self.pos);
                    }
                    digits.push(self.next_byte()?.expect("peeked disappeared"));
                }

                match self.next_byte()? {
                    Some(b'$') => {}
                    Some(_) => bail!("array length marker missing '$' at byte {}", self.pos - 1),
                    None => bail!("unexpected end of input while parsing array length marker"),
                }

                let hex = String::from_utf8(digits)?;
                canonical_hex_digits(&hex)?;
                let value = usize::from_str_radix(&hex, 16)
                    .map_err(|_| anyhow!("array length value out of range"))?;
                Ok(Some(value))
            }
            None => bail!("unexpected end of input while parsing array length marker"),
        }
    }

    pub fn next_in_array(&mut self, first: bool) -> anyhow::Result<bool> {
        if self.peek_byte()? == Some(b']') {
            self.next_byte()?;
            return Ok(false);
        }
        if first {
            return Ok(true);
        }

        match self.next_byte()? {
            Some(b',') => {}
            Some(other) => bail!(
                "expected ',' or ']' in array at byte {}, found '{}'",
                self.pos - 1,
                other as char
            ),
            None => bail!("unexpected end of input while parsing array"),
        }

        match self.peek_byte()? {
            Some(b']') => bail!("expected array element after comma at byte {}", self.pos),
            Some(_) => Ok(true),
            None => bail!("unexpected end of input after comma in array"),
        }
    }

    pub fn start_object(&mut self) -> anyhow::Result<()> {
        self.expect_byte(b'{')
    }

    pub fn next_in_object(&mut self, first: bool) -> anyhow::Result<Option<u32>> {
        if self.peek_byte()? == Some(b'}') {
            self.next_byte()?;
            return Ok(None);
        }

        if !first {
            match self.next_byte()? {
                Some(b',') => {}
                Some(other) => bail!(
                    "expected ',' or '}}' in object at byte {}, found '{}'",
                    self.pos - 1,
                    other as char
                ),
                None => bail!("unexpected end of input while parsing object"),
            }
            match self.peek_byte()? {
                Some(b'}') => bail!("expected object field after comma at byte {}", self.pos),
                Some(_) => {}
                None => bail!("unexpected end of input after comma in object"),
            }
        }

        let start = self.pos;
        let first = self
            .next_byte()?
            .ok_or_else(|| anyhow!("unexpected end of input while parsing object field id"))?;
        let mut value = u32::from(parse_lower_hex_digit(first).ok_or_else(|| {
            anyhow!("expected canonical lowercase hexadecimal digits for object field id")
        })?);

        if first == b'0' {
            match self.peek_byte()? {
                Some(b':') => {}
                Some(byte) if parse_lower_hex_digit(byte).is_some() => {
                    bail!("hex value must not contain leading zeroes")
                }
                Some(_) => {
                    bail!("expected canonical lowercase hexadecimal digits for object field id")
                }
                None => bail!("unexpected end of input while parsing object field id"),
            }
        } else {
            while let Some(byte) = self.peek_byte()? {
                if byte == b':' {
                    break;
                }
                let digit = parse_lower_hex_digit(byte).ok_or_else(|| {
                    anyhow!("expected canonical lowercase hexadecimal digits for object field id")
                })?;
                value = value
                    .checked_mul(16)
                    .and_then(|acc| acc.checked_add(u32::from(digit)))
                    .ok_or_else(|| anyhow!("object field id is out of range"))?;
                self.next_byte()?;
            }
        }

        match self.next_byte()? {
            Some(b':') => Ok(Some(value)),
            Some(other) => bail!(
                "expected ':' after object field id at byte {}, found '{}'",
                self.pos - 1,
                other as char
            ),
            None => bail!(
                "unexpected end of input after object field id at byte {}",
                start
            ),
        }
    }

    pub fn skip_value(&mut self) -> anyhow::Result<()> {
        match self.peek_byte()? {
            Some(b'"') => self.skip_string(),
            Some(b'[') => {
                let _ = self.start_array()?;
                let mut first = true;
                while self.next_in_array(first)? {
                    first = false;
                    self.skip_value()?;
                }
                Ok(())
            }
            Some(b'{') => {
                self.start_object()?;
                let mut first = true;
                while self.next_in_object(first)?.is_some() {
                    first = false;
                    self.skip_value()?;
                }
                Ok(())
            }
            Some(_) => self.skip_atom(),
            None => bail!("unexpected end of input"),
        }
    }

    fn skip_atom(&mut self) -> anyhow::Result<()> {
        let start = self.pos;
        while let Some(byte) = self.peek_byte()? {
            if is_atom_terminator(byte) {
                break;
            }
            self.next_byte()?;
        }
        if self.pos == start {
            bail!("expected atom at byte {}", start);
        }
        Ok(())
    }

    fn take_unsigned_value(
        &mut self,
        prefix: &'static str,
        tag: &'static str,
    ) -> anyhow::Result<u128> {
        self.take_exact_atom(prefix, AtomReadMode::Prefix)?;
        self.take_canonical_hex_value(tag)
    }

    fn take_signed_value(&mut self, tag: &'static str) -> anyhow::Result<(bool, u128)> {
        self.take_exact_atom(tag, AtomReadMode::Prefix)?;
        self.take_exact_atom(":", AtomReadMode::Continuation)?;
        let negative = if self.peek_byte()? == Some(b'-') {
            self.next_byte()?;
            true
        } else {
            false
        };
        Ok((negative, self.take_canonical_hex_value(tag)?))
    }

    fn take_canonical_hex_value(&mut self, tag: &'static str) -> anyhow::Result<u128> {
        let first = self
            .next_byte()?
            .ok_or_else(|| anyhow!("expected canonical lowercase hexadecimal digits for {tag}"))?;
        let first_digit = parse_lower_hex_digit(first)
            .ok_or_else(|| anyhow!("expected canonical lowercase hexadecimal digits for {tag}"))?;
        let mut value = first_digit as u128;

        if first == b'0' {
            return match self.peek_byte()? {
                Some(byte) if is_atom_terminator(byte) => Ok(0),
                Some(byte) if parse_lower_hex_digit(byte).is_some() => {
                    bail!("hex value must not contain leading zeroes")
                }
                Some(_) => bail!("expected canonical lowercase hexadecimal digits for {tag}"),
                None => Ok(0),
            };
        }

        loop {
            match self.peek_byte()? {
                Some(byte) if is_atom_terminator(byte) => return Ok(value),
                Some(byte) => {
                    let digit = parse_lower_hex_digit(byte).ok_or_else(|| {
                        anyhow!("expected canonical lowercase hexadecimal digits for {tag}")
                    })? as u128;
                    self.next_byte()?;
                    value = value
                        .checked_mul(16)
                        .and_then(|acc| acc.checked_add(digit))
                        .ok_or_else(|| anyhow!("value is out of range for {tag}"))?;
                }
                None => return Ok(value),
            }
        }
    }

    fn take_fixed_width_hex_bits(
        &mut self,
        tag: &'static str,
        width: usize,
    ) -> anyhow::Result<u64> {
        let mut value = 0u64;
        for _ in 0..width {
            let byte = self
                .next_byte()?
                .ok_or_else(|| anyhow!("expected canonical {tag} bit pattern"))?;
            let digit = parse_lower_hex_digit(byte)
                .ok_or_else(|| anyhow!("expected canonical {tag} bit pattern"))?;
            value = (value << 4) | u64::from(digit);
        }

        match self.peek_byte()? {
            Some(byte) if !is_atom_terminator(byte) => {
                bail!("expected canonical {tag} bit pattern")
            }
            _ => Ok(value),
        }
    }
}

fn is_atom_terminator(byte: u8) -> bool {
    matches!(byte, b',' | b']' | b'}')
}

fn parse_lower_hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

fn canonical_hex_digits(hex: &str) -> anyhow::Result<()> {
    if hex.is_empty() {
        bail!("hex value must contain at least one digit");
    }
    if !hex
        .bytes()
        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        bail!("hex value must use lowercase hexadecimal digits");
    }
    if hex.len() > 1 && hex.starts_with('0') {
        bail!("hex value must not contain leading zeroes");
    }
    Ok(())
}

#[inline(never)]
fn error_negative_magnitude_exceeded(
    magnitude: u128,
    limit_negative_magnitude: u128,
) -> anyhow::Error {
    anyhow!(
        "negative magnitude out of range while parsing integer: expecting max {} got {}",
        limit_negative_magnitude,
        magnitude
    )
}

#[inline(never)]
fn error_positive_magnitude_exceeded(
    magnitude: u128,
    limit_positive_magnitude: u128,
) -> anyhow::Error {
    anyhow!(
        "positive magnitude out of range while parsing integer: expecting max {} got {}",
        limit_positive_magnitude,
        magnitude
    )
}

fn convert_signed_primitive<T: TryFrom<i128>>(
    negative: bool,
    magnitude: u128,
    limit_positive_magnitude: u128,
    limit_negative_magnitude: u128,
) -> anyhow::Result<T> {
    unsafe {
        Ok(T::try_from(if negative {
            if magnitude > limit_negative_magnitude {
                return Err(error_negative_magnitude_exceeded(
                    magnitude,
                    limit_negative_magnitude,
                ));
            }
            if magnitude == (i128::MAX as u128) + 1 {
                i128::MIN
            } else {
                -(magnitude as i128)
            }
        } else {
            if magnitude > limit_positive_magnitude {
                return Err(error_positive_magnitude_exceeded(
                    magnitude,
                    limit_positive_magnitude,
                ));
            }
            magnitude as i128
        })
        .unwrap_unchecked())
    }
}

#[cfg(test)]
mod tests {
    use super::TextReader;
    use crate::ReadExt;
    use anyhow::Result;
    use kaloron::Version;
    use std::io::Cursor;
    use std::sync::Arc;

    impl<'a> ReadExt for Cursor<&'a [u8]> {
        fn facade<T>(&self) -> anyhow::Result<Arc<T>> {
            panic!("ReadExt::facade is not used by textual reader tests")
        }
    }

    #[test]
    fn test_parses_required_length_markers() -> Result<()> {
        let mut reader = TextReader::new(Version::zero(), Cursor::new(b"[2$none,none]".as_slice()));
        assert_eq!(reader.start_array()?, Some(2));
        assert!(reader.next_in_array(true)?);
        assert_eq!(reader.take_none()?, ());
        assert!(reader.next_in_array(false)?);
        assert_eq!(reader.take_none()?, ());
        assert!(!reader.next_in_array(false)?);
        Ok(())
    }

    #[test]
    fn test_parses_streamed_marker() -> Result<()> {
        let mut reader = TextReader::new(Version::zero(), Cursor::new(b"[$none]".as_slice()));
        assert_eq!(reader.start_array()?, None);
        assert!(reader.next_in_array(true)?);
        assert_eq!(reader.take_none()?, ());
        assert!(!reader.next_in_array(false)?);
        Ok(())
    }

    #[test]
    fn test_rejects_missing_marker() {
        let mut reader = TextReader::new(Version::zero(), Cursor::new(b"[none]".as_slice()));
        assert!(reader.start_array().is_err());
    }

    #[test]
    fn test_parses_numeric_atoms() -> Result<()> {
        let mut u8_reader = TextReader::new(Version::zero(), Cursor::new(b"u8:2a".as_slice()));
        assert_eq!(u8_reader.take_u8()?, 42);
        u8_reader.finish()?;

        let mut i16_reader = TextReader::new(Version::zero(), Cursor::new(b"i16:-2a".as_slice()));
        assert_eq!(i16_reader.take_i16()?, -42);
        i16_reader.finish()?;

        let mut f32_reader =
            TextReader::new(Version::zero(), Cursor::new(b"f32:3f800000".as_slice()));
        assert_eq!(f32_reader.take_f32()?.to_bits(), 0x3f80_0000);
        f32_reader.finish()?;

        let mut f64_reader = TextReader::new(
            Version::zero(),
            Cursor::new(b"f64:8000000000000000".as_slice()),
        );
        assert_eq!(f64_reader.take_f64()?.to_bits(), 0x8000_0000_0000_0000);
        f64_reader.finish()?;

        let mut i128_reader = TextReader::new(
            Version::zero(),
            Cursor::new(b"i128:-80000000000000000000000000000000".as_slice()),
        );
        assert_eq!(i128_reader.take_i128()?, i128::MIN);
        i128_reader.finish()?;
        Ok(())
    }

    #[test]
    fn test_rejects_non_canonical_numeric_atoms() {
        let mut leading_zero = TextReader::new(Version::zero(), Cursor::new(b"u8:01".as_slice()));
        assert!(leading_zero.take_u8().is_err());

        let mut whitespace = TextReader::new(Version::zero(), Cursor::new(b"i8: -1".as_slice()));
        assert!(whitespace.take_i8().is_err());

        let mut uppercase =
            TextReader::new(Version::zero(), Cursor::new(b"f32:3F800000".as_slice()));
        assert!(uppercase.take_f32().is_err());

        let mut positive_overflow = TextReader::new(
            Version::zero(),
            Cursor::new(b"i128:80000000000000000000000000000000".as_slice()),
        );
        assert!(positive_overflow.take_i128().is_err());
    }

    #[test]
    fn test_signed_magnitude_limits_follow_sign() -> Result<()> {
        let mut negative_min = TextReader::new(Version::zero(), Cursor::new(b"i8:-80".as_slice()));
        assert_eq!(negative_min.take_i8()?, i8::MIN);
        negative_min.finish()?;

        let mut negative_overflow =
            TextReader::new(Version::zero(), Cursor::new(b"i8:-81".as_slice()));
        assert!(negative_overflow.take_i8().is_err());

        let mut positive_max = TextReader::new(Version::zero(), Cursor::new(b"i8:7f".as_slice()));
        assert_eq!(positive_max.take_i8()?, i8::MAX);
        positive_max.finish()?;

        let mut positive_overflow =
            TextReader::new(Version::zero(), Cursor::new(b"i8:80".as_slice()));
        assert!(positive_overflow.take_i8().is_err());
        Ok(())
    }

    #[test]
    fn test_rejects_whitespace_anywhere_outside_strings() {
        let mut leading = TextReader::new(Version::zero(), Cursor::new(b" true".as_slice()));
        assert!(leading.take_bool().is_err());

        let mut trailing = TextReader::new(Version::zero(), Cursor::new(b"true ".as_slice()));
        assert!(trailing.take_bool().is_err());

        let mut array = TextReader::new(Version::zero(), Cursor::new(b"[2$none, none]".as_slice()));
        assert_eq!(array.start_array().unwrap(), Some(2));
        assert!(array.next_in_array(true).unwrap());
        assert!(array.take_none().is_ok());
        assert!(array.next_in_array(false).unwrap());
        assert!(array.take_none().is_err());

        let mut object = TextReader::new(Version::zero(), Cursor::new(b"{0 :none}".as_slice()));
        object.start_object().unwrap();
        assert!(object.next_in_object(true).is_err());
    }
}
