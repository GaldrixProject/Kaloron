// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use super::super::{TextReader, TextWriter};
use super::common::*;
use crate::{ReadExt, WriteExt};
use anyhow::bail;
use kaloron::*;

#[derive(Copy, Clone)]
enum MapPairSide {
    Key,
    Value,
}

pub(crate) fn encode_map<W: WriteExt>(
    send: &impl MapSend,
    writer: &mut TextWriter<W>,
) -> anyhow::Result<()> {
    let writer = writer as *mut TextWriter<W>;

    unsafe { &mut *writer }.start_array(send.len())?;

    let mut first = true;
    while send.has_next() {
        unsafe { &mut *writer }.next_in_array(first)?;
        first = false;

        unsafe { &mut *writer }.start_array(Some(2))?;
        let mut key_visitor = MapPairSendVisitor::new(writer, MapPairSide::Key);
        let mut value_visitor = MapPairSendVisitor::new(writer, MapPairSide::Value);
        send.visit_next(&mut key_visitor, &mut value_visitor)?;
        unsafe { &mut *writer }.finish_array()?;
    }

    unsafe { &mut *writer }.finish_array()
}

struct MapPairSendVisitor<W: WriteExt> {
    writer: *mut TextWriter<W>,
    side: MapPairSide,
}

impl<W: WriteExt> MapPairSendVisitor<W> {
    fn new(writer: *mut TextWriter<W>, side: MapPairSide) -> Self {
        Self { writer, side }
    }
}

impl<W: WriteExt> SendVisitor for MapPairSendVisitor<W> {
    fn visit<T: TypeShape>(&mut self, value: &T) -> anyhow::Result<()> {
        let writer = unsafe { &mut *self.writer };
        match self.side {
            MapPairSide::Key => writer.next_in_array(true)?,
            MapPairSide::Value => writer.next_in_array(false)?,
        }
        encode_inner(value, writer)
    }
}

struct MapPairRecvVisitor<R: ReadExt> {
    reader: *mut TextReader<R>,
    side: MapPairSide,
}

impl<R: ReadExt> MapPairRecvVisitor<R> {
    fn new(reader: *mut TextReader<R>, side: MapPairSide) -> Self {
        Self { reader, side }
    }
}

impl<R: ReadExt> RecvVisitor for MapPairRecvVisitor<R> {
    fn visit<T: TypeShape>(&mut self) -> anyhow::Result<T> {
        let reader = unsafe { &mut *self.reader };
        match self.side {
            MapPairSide::Key => {
                if !reader.next_in_array(true)? {
                    bail!("map entry is missing a key");
                }
                decode_inner(reader)
            }
            MapPairSide::Value => {
                if !reader.next_in_array(false)? {
                    bail!("map entry is missing a value");
                }
                let value = decode_inner(reader)?;
                if reader.next_in_array(false)? {
                    bail!("map entry has more than two elements");
                }
                Ok(value)
            }
        }
    }
}

pub(crate) fn decode_map<R: ReadExt>(
    recv: &mut impl MapRecv,
    reader: &mut TextReader<R>,
) -> anyhow::Result<()> {
    let size = reader.start_array()?;
    if let Some(size) = size {
        recv.len(size)?;
    }

    let reader = reader as *mut TextReader<R>;

    let mut first = true;
    while unsafe { &mut *reader }.next_in_array(first)? {
        first = false;

        let pair_len = unsafe { &mut *reader }.start_array()?;
        if let Some(got) = pair_len
            && got != 2
        {
            bail!("map entry length mismatch: expected 2, got {got}");
        }

        let mut key = MapPairRecvVisitor::new(reader, MapPairSide::Key);
        let mut value = MapPairRecvVisitor::new(reader, MapPairSide::Value);
        recv.visit_next(&mut key, &mut value)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use kaloron::Version;
    use std::collections::BTreeMap;

    #[test]
    fn test_map_round_trip_uses_pair_arrays() -> Result<()> {
        let value = BTreeMap::from([(1u8, 10u8)]);
        let out = super::super::assist_encode_test_value(Version::zero(), &value)?;
        assert_eq!(String::from_utf8(out.clone()).unwrap(), "[1$[2$u8:1,u8:a]]");

        let decoded: BTreeMap<u8, u8> = super::super::assist_decode_test_value(Version::zero(), &out)?;
        assert_eq!(decoded, value);
        Ok(())
    }
}
