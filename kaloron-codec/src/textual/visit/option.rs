// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use super::super::{TextReader, TextWriter};
use super::common::*;
use crate::{ReadExt, WriteExt};
use kaloron::*;

pub(crate) fn encode_option<W: WriteExt>(
    send: &impl OptionSend,
    writer: &mut TextWriter<W>,
) -> anyhow::Result<()> {
    if send.is_some() {
        send.visit(&mut RecursiveSendVisitor::new(writer))
    } else {
        writer.write_none()
    }
}

pub(crate) fn decode_option<R: ReadExt>(
    recv: &mut impl OptionRecv,
    reader: &mut TextReader<R>,
) -> anyhow::Result<()> {
    if reader.maybe_take_none()? {
        recv.as_some(false)
    } else {
        recv.as_some(true)?;
        recv.visit(&mut RecursiveRecvVisitor::new(reader))
    }
}
