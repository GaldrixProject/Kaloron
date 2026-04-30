// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use crate::WriteExt;
use kaloron::Version;

pub struct TextWriter<W: WriteExt> {
    version: Version,
    inner: W,
}

impl<W: WriteExt> TextWriter<W> {
    pub fn new(version: Version, inner: W) -> Self {
        Self { version, inner }
    }

    pub fn version(&self) -> Version {
        self.version
    }

    pub fn write_value_bytes(&mut self, bytes: &[u8]) -> anyhow::Result<()> {
        self.inner.write_all(bytes)?;
        Ok(())
    }

    pub fn write_none(&mut self) -> anyhow::Result<()> {
        self.write_value_bytes(b"none")
    }

    pub fn write_unit(&mut self) -> anyhow::Result<()> {
        self.write_value_bytes(b"unit")
    }

    pub fn write_bool(&mut self, value: bool) -> anyhow::Result<()> {
        self.write_value_bytes(if value { b"true" } else { b"false" })
    }

    pub fn write_char(&mut self, value: char) -> anyhow::Result<()> {
        let mut buf = [0u8; 4];
        self.write_string(value.encode_utf8(&mut buf))
    }

    pub fn write_string(&mut self, value: &str) -> anyhow::Result<()> {
        serde_json::to_writer(&mut self.inner, value)?;
        Ok(())
    }

    pub fn write_u8(&mut self, value: u8) -> anyhow::Result<()> {
        self.inner.write_fmt(format_args!("u8:{:x}", value))?;
        Ok(())
    }

    pub fn write_u16(&mut self, value: u16) -> anyhow::Result<()> {
        self.inner.write_fmt(format_args!("u16:{:x}", value))?;
        Ok(())
    }

    pub fn write_u32(&mut self, value: u32) -> anyhow::Result<()> {
        self.inner.write_fmt(format_args!("u32:{:x}", value))?;
        Ok(())
    }

    pub fn write_u64(&mut self, value: u64) -> anyhow::Result<()> {
        self.inner.write_fmt(format_args!("u64:{:x}", value))?;
        Ok(())
    }

    pub fn write_u128(&mut self, value: u128) -> anyhow::Result<()> {
        self.inner.write_fmt(format_args!("u128:{:x}", value))?;
        Ok(())
    }

    pub fn write_i8(&mut self, value: i8) -> anyhow::Result<()> {
        if value < 0 {
            self.inner
                .write_fmt(format_args!("i8:-{:x}", value.unsigned_abs()))?;
        } else {
            self.inner.write_fmt(format_args!("i8:{:x}", value))?;
        }
        Ok(())
    }

    pub fn write_i16(&mut self, value: i16) -> anyhow::Result<()> {
        if value < 0 {
            self.inner
                .write_fmt(format_args!("i16:-{:x}", value.unsigned_abs()))?;
        } else {
            self.inner.write_fmt(format_args!("i16:{:x}", value))?;
        }
        Ok(())
    }

    pub fn write_i32(&mut self, value: i32) -> anyhow::Result<()> {
        if value < 0 {
            self.inner
                .write_fmt(format_args!("i32:-{:x}", value.unsigned_abs()))?;
        } else {
            self.inner.write_fmt(format_args!("i32:{:x}", value))?;
        }
        Ok(())
    }

    pub fn write_i64(&mut self, value: i64) -> anyhow::Result<()> {
        if value < 0 {
            self.inner
                .write_fmt(format_args!("i64:-{:x}", value.unsigned_abs()))?;
        } else {
            self.inner.write_fmt(format_args!("i64:{:x}", value))?;
        }
        Ok(())
    }

    pub fn write_i128(&mut self, value: i128) -> anyhow::Result<()> {
        if value < 0 {
            self.inner
                .write_fmt(format_args!("i128:-{:x}", value.unsigned_abs()))?;
        } else {
            self.inner.write_fmt(format_args!("i128:{:x}", value))?;
        }
        Ok(())
    }

    pub fn write_f32(&mut self, value: f32) -> anyhow::Result<()> {
        self.inner
            .write_fmt(format_args!("f32:{:08x}", value.to_bits()))?;
        Ok(())
    }

    pub fn write_f64(&mut self, value: f64) -> anyhow::Result<()> {
        self.inner
            .write_fmt(format_args!("f64:{:016x}", value.to_bits()))?;
        Ok(())
    }

    pub fn start_array(&mut self, len: Option<usize>) -> anyhow::Result<()> {
        // Write the opening bracket and the length marker.
        self.write_value_bytes(b"[")?;
        match len {
            Some(n) => {
                // canonical lowercase hex without leading zeros
                self.inner.write_fmt(format_args!("{:x}$", n))?;
            }
            None => {
                // streamed / unknown length marker is just '$'
                self.write_value_bytes(b"$")?;
            }
        }
        Ok(())
    }

    pub fn next_in_array(&mut self, first: bool) -> anyhow::Result<()> {
        if !first {
            self.write_value_bytes(b",")?;
        }
        Ok(())
    }

    pub fn finish_array(&mut self) -> anyhow::Result<()> {
        self.write_value_bytes(b"]")
    }

    pub fn start_object(&mut self) -> anyhow::Result<()> {
        self.write_value_bytes(b"{")
    }

    pub fn next_in_object(&mut self, first: bool, id: u32) -> anyhow::Result<()> {
        if !first {
            self.write_value_bytes(b",")?;
        }
        self.inner.write_fmt(format_args!("{:x}", id))?;
        self.write_value_bytes(b":")
    }

    pub fn finish_object(&mut self) -> anyhow::Result<()> {
        self.write_value_bytes(b"}")
    }
}

#[cfg(test)]
mod tests {
    use super::TextWriter;
    use crate::WriteExt;
    use anyhow::Result;
    use kaloron::Version;
    use std::sync::Arc;

    impl WriteExt for &mut Vec<u8> {
        fn facade<T>(&self) -> anyhow::Result<Arc<T>> {
            panic!("WriteExt::facade is not used by textual writer tests")
        }
    }

    fn assist_render(write: impl FnOnce(&mut TextWriter<&mut Vec<u8>>) -> Result<()>) -> Result<String> {
        let mut out = Vec::new();
        let mut writer = TextWriter::new(Version::zero(), &mut out);
        write(&mut writer)?;
        Ok(String::from_utf8(out).expect("utf-8"))
    }

    #[test]
    fn test_writes_canonical_scalar_forms() -> Result<()> {
        assert_eq!(assist_render(|writer| writer.write_none())?, "none");
        assert_eq!(assist_render(|writer| writer.write_unit())?, "unit");
        assert_eq!(assist_render(|writer| writer.write_bool(true))?, "true");
        assert_eq!(assist_render(|writer| writer.write_char('ß'))?, "\"ß\"");
        assert_eq!(
            assist_render(|writer| writer.write_string("hello\nworld"))?,
            "\"hello\\nworld\""
        );
        assert_eq!(assist_render(|writer| writer.write_u8(42))?, "u8:2a");
        assert_eq!(assist_render(|writer| writer.write_i64(-42))?, "i64:-2a");
        assert_eq!(assist_render(|writer| writer.write_f32(1.0))?, "f32:3f800000");
        assert_eq!(
            assist_render(|writer| writer.write_f64(-0.0))?,
            "f64:8000000000000000"
        );
        Ok(())
    }

    #[test]
    fn test_writes_array_and_object_structure() -> Result<()> {
        assert_eq!(
            assist_render(|writer| {
                writer.start_array(Some(2))?;
                writer.next_in_array(true)?;
                writer.write_u8(1)?;
                writer.next_in_array(false)?;
                writer.write_string("Ada")?;
                writer.finish_array()
            })?,
            "[2$u8:1,\"Ada\"]"
        );

        assert_eq!(
            assist_render(|writer| {
                writer.start_array(None)?;
                writer.next_in_array(true)?;
                writer.write_none()?;
                writer.next_in_array(false)?;
                writer.write_none()?;
                writer.finish_array()
            })?,
            "[$none,none]"
        );

        assert_eq!(
            assist_render(|writer| {
                writer.start_object()?;
                writer.next_in_object(true, 0)?;
                writer.write_string("Ok")?;
                writer.next_in_object(false, 1)?;
                writer.write_u8(7)?;
                writer.finish_object()
            })?,
            "{0:\"Ok\",1:u8:7}"
        );
        Ok(())
    }
}
