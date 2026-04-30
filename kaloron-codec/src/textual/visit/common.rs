// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use super::super::{TextReader, TextWriter};
use super::enums::*;
use super::maps::*;
use super::named_struct::*;
use super::option::*;
use super::primitives::*;
use super::sequence::*;
use super::tuples::*;
use crate::{ReadExt, WriteExt};
use anyhow::bail;
use kaloron::*;

pub(crate) struct RecursiveSendVisitor<'writer, W: WriteExt> {
    writer: &'writer mut TextWriter<W>,
}

impl<'writer, W: WriteExt> RecursiveSendVisitor<'writer, W> {
    pub(crate) fn new(writer: &'writer mut TextWriter<W>) -> Self {
        Self { writer }
    }
}

impl<'writer, W: WriteExt> SendVisitor for RecursiveSendVisitor<'writer, W> {
    fn visit<T: TypeShape>(&mut self, value: &T) -> anyhow::Result<()> {
        encode_inner(value, self.writer)
    }
}

pub(crate) struct RecursiveRecvVisitor<'reader, R: ReadExt> {
    reader: &'reader mut TextReader<R>,
}

impl<'reader, R: ReadExt> RecursiveRecvVisitor<'reader, R> {
    pub(crate) fn new(reader: &'reader mut TextReader<R>) -> Self {
        Self { reader }
    }
}

impl<'reader, R: ReadExt> RecvVisitor for RecursiveRecvVisitor<'reader, R> {
    fn visit<T: TypeShape>(&mut self) -> anyhow::Result<T> {
        decode_inner(self.reader)
    }
}

pub(crate) fn encode_inner<T: TypeShape, W: WriteExt>(
    o: &T,
    writer: &mut TextWriter<W>,
) -> anyhow::Result<()> {
    struct Accept<'writer, W: WriteExt> {
        writer: &'writer mut TextWriter<W>,
        schema: &'static Schema<'static>,
    }

    impl<'writer, W: WriteExt> SendAccept for Accept<'writer, W> {
        fn accept_primitive(&mut self, send: &impl PrimitiveSend) -> anyhow::Result<()> {
            send.visit(&mut PrimitiveVisitorSendImpl::new(self.writer))
        }

        fn accept_option(&mut self, send: &impl OptionSend) -> anyhow::Result<()> {
            encode_option(send, self.writer)
        }

        fn accept_sequence(&mut self, send: &impl SequenceSend) -> anyhow::Result<()> {
            encode_sequence(send, self.writer)
        }

        fn accept_tuple(&mut self, send: &impl TupleSend) -> anyhow::Result<()> {
            match self.schema {
                Schema::Tuple(elems) => encode_tuple(send, self.writer, elems.len()),
                _ => bail!("schema mismatch: expected tuple schema"),
            }
        }

        fn accept_map(&mut self, send: &impl MapSend) -> anyhow::Result<()> {
            encode_map(send, self.writer)
        }

        fn accept_newtype_struct(&mut self, send: &impl NewTypeSend) -> anyhow::Result<()> {
            match self.schema {
                Schema::NewTypeStruct(_) => send.visit(&mut RecursiveSendVisitor::new(self.writer)),
                _ => bail!("schema mismatch: expected newtype struct schema"),
            }
        }

        fn accept_tuple_struct(&mut self, send: &impl TupleSend) -> anyhow::Result<()> {
            match self.schema {
                Schema::TupleStruct(tuple) => encode_tuple(send, self.writer, tuple.elems.len()),
                _ => bail!("schema mismatch: expected tuple struct schema"),
            }
        }

        fn accept_named_struct(&mut self, send: &impl TupleSend) -> anyhow::Result<()> {
            match self.schema {
                Schema::Named(named) => encode_named_struct(send, self.writer, named),
                _ => bail!("schema mismatch: expected named struct schema"),
            }
        }

        fn accept_enum(&mut self, send: &impl EnumSend) -> anyhow::Result<()> {
            match self.schema {
                Schema::Enum(enum_schema) => encode_enum(send, self.writer, enum_schema),
                _ => bail!("schema mismatch: expected enum schema"),
            }
        }
    }

    let version = writer.version();
    T::send(
        o,
        version,
        &mut Accept {
            writer,
            schema: T::SCHEMA,
        },
    )
}

pub(crate) fn decode_inner<T: TypeShape, R: ReadExt>(
    reader: &mut TextReader<R>,
) -> anyhow::Result<T> {
    struct Accept<'reader, R: ReadExt> {
        reader: &'reader mut TextReader<R>,
        schema: &'static Schema<'static>,
    }

    impl<R: ReadExt> RecvAccept for Accept<'_, R> {
        fn accept_primitive(&mut self, recv: &mut impl PrimitiveRecv) -> anyhow::Result<()> {
            recv.visit(&mut PrimitiveVisitorRecvImpl::new(self.reader))
        }

        fn accept_option(&mut self, recv: &mut impl OptionRecv) -> anyhow::Result<()> {
            decode_option(recv, self.reader)
        }

        fn accept_sequence(&mut self, recv: &mut impl SequenceRecv) -> anyhow::Result<()> {
            decode_sequence(recv, self.reader)
        }

        fn accept_tuple(&mut self, recv: &mut impl TupleRecv) -> anyhow::Result<()> {
            match self.schema {
                Schema::Tuple(elems) => decode_tuple(recv, self.reader, elems.len()),
                _ => bail!("schema mismatch: expected tuple schema"),
            }
        }

        fn accept_map(&mut self, recv: &mut impl MapRecv) -> anyhow::Result<()> {
            decode_map(recv, self.reader)
        }

        fn accept_newtype_struct(&mut self, recv: &mut impl NewTypeRecv) -> anyhow::Result<()> {
            match self.schema {
                Schema::NewTypeStruct(_) => recv.visit(&mut RecursiveRecvVisitor::new(self.reader)),
                _ => bail!("schema mismatch: expected newtype struct schema"),
            }
        }

        fn accept_tuple_struct(&mut self, recv: &mut impl TupleRecv) -> anyhow::Result<()> {
            match self.schema {
                Schema::TupleStruct(tuple) => decode_tuple(recv, self.reader, tuple.elems.len()),
                _ => bail!("schema mismatch: expected tuple struct schema"),
            }
        }

        fn accept_named_struct(&mut self, recv: &mut impl TupleRecv) -> anyhow::Result<()> {
            match self.schema {
                Schema::Named(named) => decode_named_struct(recv, self.reader, named),
                _ => bail!("schema mismatch: expected named struct schema"),
            }
        }

        fn accept_enum(&mut self, recv: &mut impl EnumRecv) -> anyhow::Result<()> {
            match self.schema {
                Schema::Enum(enum_schema) => decode_enum(recv, self.reader, enum_schema),
                _ => bail!("schema mismatch: expected enum schema"),
            }
        }
    }

    let version = reader.version();
    let value = T::recv(
        version,
        &mut Accept {
            reader,
            schema: T::SCHEMA,
        },
    )?;
    Ok(value)
}
