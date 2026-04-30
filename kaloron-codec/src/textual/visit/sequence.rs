// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use super::super::{TextReader, TextWriter};
use super::common::*;
use crate::{ReadExt, WriteExt};
use kaloron::*;

pub(crate) fn encode_sequence<W: WriteExt>(
    send: &impl SequenceSend,
    writer: &mut TextWriter<W>,
) -> anyhow::Result<()> {
    writer.start_array(send.len())?;
    let mut first = true;
    while send.has_next() {
        writer.next_in_array(first)?;
        first = false;
        send.visit_next(&mut RecursiveSendVisitor::new(writer))?;
    }
    writer.finish_array()
}

pub(crate) fn decode_sequence<R: ReadExt>(
    recv: &mut impl SequenceRecv,
    reader: &mut TextReader<R>,
) -> anyhow::Result<()> {
    let size = reader.start_array()?;
    recv.len(size)?;

    let mut first = true;
    while reader.next_in_array(first)? {
        first = false;
        recv.visit_next(&mut RecursiveRecvVisitor::new(reader))?;
    }

    Ok(())
}
