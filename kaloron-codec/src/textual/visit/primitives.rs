// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use super::super::{TextReader, TextWriter};
use crate::{ReadExt, WriteExt};
use anyhow::anyhow;
use kaloron::*;

pub(crate) struct PrimitiveVisitorSendImpl<'writer, W: WriteExt> {
    writer: &'writer mut TextWriter<W>,
}

impl<'writer, W: WriteExt> PrimitiveVisitorSendImpl<'writer, W> {
    pub(crate) fn new(writer: &'writer mut TextWriter<W>) -> Self {
        Self { writer }
    }
}

impl<'writer, W: WriteExt> PrimitiveSendVisitor for PrimitiveVisitorSendImpl<'writer, W> {
    fn visit_bool(&mut self, v: &bool) -> anyhow::Result<()> {
        self.writer.write_bool(*v)
    }

    fn visit_i8(&mut self, v: &i8) -> anyhow::Result<()> {
        self.writer.write_i8(*v)
    }

    fn visit_i16(&mut self, v: &i16) -> anyhow::Result<()> {
        self.writer.write_i16(*v)
    }

    fn visit_i32(&mut self, v: &i32) -> anyhow::Result<()> {
        self.writer.write_i32(*v)
    }

    fn visit_i64(&mut self, v: &i64) -> anyhow::Result<()> {
        self.writer.write_i64(*v)
    }

    fn visit_i128(&mut self, v: &i128) -> anyhow::Result<()> {
        self.writer.write_i128(*v)
    }

    fn visit_u8(&mut self, v: &u8) -> anyhow::Result<()> {
        self.writer.write_u8(*v)
    }

    fn visit_u16(&mut self, v: &u16) -> anyhow::Result<()> {
        self.writer.write_u16(*v)
    }

    fn visit_u32(&mut self, v: &u32) -> anyhow::Result<()> {
        self.writer.write_u32(*v)
    }

    fn visit_u64(&mut self, v: &u64) -> anyhow::Result<()> {
        self.writer.write_u64(*v)
    }

    fn visit_u128(&mut self, v: &u128) -> anyhow::Result<()> {
        self.writer.write_u128(*v)
    }

    fn visit_f32(&mut self, v: &f32) -> anyhow::Result<()> {
        self.writer.write_f32(*v)
    }

    fn visit_f64(&mut self, v: &f64) -> anyhow::Result<()> {
        self.writer.write_f64(*v)
    }

    fn visit_char(&mut self, v: &char) -> anyhow::Result<()> {
        self.writer.write_char(*v)
    }

    fn visit_string(&mut self, v: &String) -> anyhow::Result<()> {
        self.writer.write_string(v)
    }

    #[cfg(unix)]
    fn visit_file(&mut self, v: &std::os::fd::OwnedFd) -> anyhow::Result<()> {
        use std::os::fd::AsRawFd;

        self.writer.write_u64(v.as_raw_fd() as u64)
    }
}

pub(crate) struct PrimitiveVisitorRecvImpl<'reader, R: ReadExt> {
    reader: &'reader mut TextReader<R>,
}

impl<'reader, R: ReadExt> PrimitiveVisitorRecvImpl<'reader, R> {
    pub(crate) fn new(reader: &'reader mut TextReader<R>) -> Self {
        Self { reader }
    }
}

impl<'reader, R: ReadExt> PrimitiveRecvVisitor for PrimitiveVisitorRecvImpl<'reader, R> {
    fn visit_bool(&mut self) -> anyhow::Result<bool> {
        self.reader.take_bool()
    }

    fn visit_i8(&mut self) -> anyhow::Result<i8> {
        self.reader.take_i8()
    }

    fn visit_i16(&mut self) -> anyhow::Result<i16> {
        self.reader.take_i16()
    }

    fn visit_i32(&mut self) -> anyhow::Result<i32> {
        self.reader.take_i32()
    }

    fn visit_i64(&mut self) -> anyhow::Result<i64> {
        self.reader.take_i64()
    }

    fn visit_i128(&mut self) -> anyhow::Result<i128> {
        self.reader.take_i128()
    }

    fn visit_u8(&mut self) -> anyhow::Result<u8> {
        self.reader.take_u8()
    }

    fn visit_u16(&mut self) -> anyhow::Result<u16> {
        self.reader.take_u16()
    }

    fn visit_u32(&mut self) -> anyhow::Result<u32> {
        self.reader.take_u32()
    }

    fn visit_u64(&mut self) -> anyhow::Result<u64> {
        self.reader.take_u64()
    }

    fn visit_u128(&mut self) -> anyhow::Result<u128> {
        self.reader.take_u128()
    }

    fn visit_f32(&mut self) -> anyhow::Result<f32> {
        self.reader.take_f32()
    }

    fn visit_f64(&mut self) -> anyhow::Result<f64> {
        self.reader.take_f64()
    }

    fn visit_char(&mut self) -> anyhow::Result<char> {
        self.reader.take_char()
    }

    fn visit_string(&mut self) -> anyhow::Result<String> {
        self.reader.take_string()
    }

    #[cfg(unix)]
    fn visit_file(&mut self) -> anyhow::Result<std::os::fd::OwnedFd> {
        use std::convert::TryFrom;
        use std::os::fd::{FromRawFd, OwnedFd};

        let fd = i32::try_from(self.reader.take_u64()?)
            .map_err(|_| anyhow!("value is out of range for file descriptor"))?;
        // SAFETY: the decoded integer is treated as an owned file descriptor
        // exactly as it is represented on the wire.
        Ok(unsafe { OwnedFd::from_raw_fd(fd) })
    }
}
