// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use kaloron::{TypeShape, Version};
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::fd::OwnedFd;
use std::sync::Arc;

pub mod textual;

pub use textual::{TextReader, TextWriter};

#[cfg(unix)]
pub trait FdSend {
    fn send(&self, fd: OwnedFd) -> u64;
}

#[cfg(unix)]
pub trait FdRecv {
    fn recv(&self, id: u64) -> OwnedFd;
}

pub trait WriteExt: Write {
    fn facade<T>(&self) -> anyhow::Result<Arc<T>>;
}

pub trait ReadExt: Read {
    fn facade<T>(&self) -> anyhow::Result<Arc<T>>;
}

impl<W: WriteExt + ?Sized> WriteExt for &mut W {
    fn facade<T>(&self) -> anyhow::Result<Arc<T>> {
        W::facade::<T>(self)
    }
}

impl<R: ReadExt + ?Sized> ReadExt for &mut R {
    fn facade<T>(&self) -> anyhow::Result<Arc<T>> {
        R::facade::<T>(self)
    }
}

pub trait Codec {
    fn bound<T: TypeShape>(v: Version) -> Option<usize>;

    fn encode<T: TypeShape>(v: Version, o: &T, w: &mut impl WriteExt) -> anyhow::Result<()>;

    fn decode<T: TypeShape>(v: Version, r: &mut impl ReadExt) -> anyhow::Result<T>;
}

pub struct TextualCodec;

impl Codec for TextualCodec {
    fn bound<T: TypeShape>(v: Version) -> Option<usize> {
        textual::bound(T::SCHEMA, v)
    }

    fn encode<T: TypeShape>(v: Version, o: &T, w: &mut impl WriteExt) -> anyhow::Result<()> {
        textual::encode(v, o, w)
    }

    fn decode<T: TypeShape>(v: Version, r: &mut impl ReadExt) -> anyhow::Result<T> {
        textual::decode(v, r)
    }
}
