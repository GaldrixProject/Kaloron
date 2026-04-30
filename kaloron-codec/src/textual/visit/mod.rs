// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

mod common;
mod enums;
mod maps;
mod named_struct;
mod option;
mod primitives;
mod sequence;
mod tuples;

#[cfg(test)]
use crate::{ReadExt, WriteExt};
#[cfg(test)]
use kaloron::{TypeShape, Version};
#[cfg(test)]
use std::io::{Cursor, Read, Write};
#[cfg(test)]
use std::sync::Arc;

pub(super) use common::*;

#[cfg(test)]
struct MockTestReader<'a> {
    inner: Cursor<&'a [u8]>,
}

#[cfg(test)]
impl<'a> MockTestReader<'a> {
    fn new(input: &'a [u8]) -> Self {
        Self {
            inner: Cursor::new(input),
        }
    }
}

#[cfg(test)]
impl Read for MockTestReader<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}

#[cfg(test)]
impl ReadExt for MockTestReader<'_> {
    fn facade<T>(&self) -> anyhow::Result<Arc<T>> {
        panic!("ReadExt::facade is not used by textual codec tests")
    }
}

#[cfg(test)]
struct MockTestWriter<'a> {
    inner: &'a mut Vec<u8>,
}

#[cfg(test)]
impl Write for MockTestWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

#[cfg(test)]
impl WriteExt for MockTestWriter<'_> {
    fn facade<T>(&self) -> anyhow::Result<Arc<T>> {
        panic!("WriteExt::facade is not used by textual codec tests")
    }
}

#[cfg(test)]
fn assist_encode_test_value<T: TypeShape>(version: Version, value: &T) -> anyhow::Result<Vec<u8>> {
    let mut out = Vec::new();
    let mut writer = MockTestWriter { inner: &mut out };
    super::encode(version, value, &mut writer)?;
    Ok(out)
}

#[cfg(test)]
fn assist_decode_test_value<T: TypeShape>(version: Version, input: &[u8]) -> anyhow::Result<T> {
    let mut reader = MockTestReader::new(input);
    super::decode(version, &mut reader)
}
