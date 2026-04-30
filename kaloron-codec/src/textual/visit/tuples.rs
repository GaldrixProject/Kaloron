// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use super::super::{TextReader, TextWriter};
use super::common::*;
use crate::{ReadExt, WriteExt};
use anyhow::bail;
use kaloron::*;

pub(crate) fn encode_tuple<W: WriteExt>(
    send: &impl TupleSend,
    writer: &mut TextWriter<W>,
    len: usize,
) -> anyhow::Result<()> {
    writer.start_array(Some(len))?;

    for index in 0..len {
        writer.next_in_array(index == 0)?;
        send.visit(index as isize, &mut RecursiveSendVisitor::new(writer))?;
    }

    writer.finish_array()
}

pub(crate) fn decode_tuple<R: ReadExt>(
    recv: &mut impl TupleRecv,
    reader: &mut TextReader<R>,
    len: usize,
) -> anyhow::Result<()> {
    let marker = reader.start_array()?;
    if let Some(got) = marker
        && got != len
    {
        bail!("tuple length mismatch: expected {len}, got {got}");
    }

    let mut actual = 0usize;
    let mut first = true;
    while reader.next_in_array(first)? {
        first = false;
        if actual >= len {
            bail!("tuple length mismatch: expected {len}, got more than {len}");
        }
        recv.visit(actual as isize, &mut RecursiveRecvVisitor::new(reader))?;
        actual += 1;
    }

    if actual != len {
        bail!("tuple length mismatch: expected {len}, got {actual}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use kaloron::Version;

    #[test]
    fn test_tuple_round_trip_uses_array_shape() -> Result<()> {
        let value = (1u8, 2u16);
        let out = super::super::assist_encode_test_value(Version::zero(), &value)?;
        assert_eq!(String::from_utf8(out.clone()).unwrap(), "[2$u8:1,u16:2]");

        let decoded: (u8, u16) = super::super::assist_decode_test_value(Version::zero(), &out)?;
        assert_eq!(decoded, value);
        Ok(())
    }
}
