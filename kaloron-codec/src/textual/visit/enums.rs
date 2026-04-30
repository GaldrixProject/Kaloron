// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use super::super::{TextReader, TextWriter};
use super::common::*;
use super::named_struct::*;
use super::tuples::*;
use crate::{ReadExt, WriteExt};
use anyhow::bail;
use kaloron::*;

fn active_variant_indices(schema: &EnumSchema<'_>, version: Version) -> anyhow::Result<Vec<usize>> {
    match schema.active_variants_at(version) {
        Some(states) => {
            if states.len() != schema.variants.len() {
                bail!("schema mismatch: enum activation table length does not match variant count");
            }

            Ok(states
                .iter()
                .enumerate()
                .filter_map(|(index, state)| match state {
                    ActivationState::Inactive => None,
                    ActivationState::Active | ActivationState::Deprecated => Some(index),
                })
                .collect())
        }
        None => Ok((0..schema.variants.len()).collect()),
    }
}

fn variant_for_send<'a>(
    schema: &'a EnumSchema<'a>,
    active_indices: &[usize],
    index: isize,
    version: Version,
) -> anyhow::Result<&'a Variant<'a>> {
    if index < 0 {
        bail!("unknown enum variant index: {index}");
    }

    let index = index as usize;
    let variant = schema
        .variants
        .get(index)
        .ok_or_else(|| anyhow::anyhow!("unknown enum variant index: {}", index))?;

    if !active_indices.contains(&index) {
        bail!(
            "enum variant index {} is inactive at version {}",
            index,
            version
        );
    }

    Ok(variant)
}

fn variant_for_recv<'a>(
    schema: &'a EnumSchema<'a>,
    active_indices: &[usize],
    wire_id: u32,
    version: Version,
) -> anyhow::Result<(usize, &'a Variant<'a>)> {
    let index = schema
        .variants
        .iter()
        .position(|variant| variant.id == wire_id)
        .ok_or_else(|| anyhow::anyhow!("unknown enum variant id {:x}", wire_id))?;

    if !active_indices.contains(&index) {
        bail!(
            "enum variant id {:x} is inactive at version {}",
            wire_id,
            version
        );
    }

    Ok((index, &schema.variants[index]))
}

fn write_variant_object<W: WriteExt, F>(
    writer: &mut TextWriter<W>,
    id: u32,
    payload: F,
) -> anyhow::Result<()>
where
    F: FnOnce(&mut TextWriter<W>) -> anyhow::Result<()>,
{
    writer.start_object()?;
    writer.next_in_object(true, id)?;
    payload(writer)?;
    writer.finish_object()
}

struct EmptyEnumRecvAccept;

impl EnumRecvAccept for EmptyEnumRecvAccept {
    fn accept_newtype(&mut self, _visitor: &mut impl NewTypeRecv) -> anyhow::Result<()> {
        bail!("schema mismatch: expected unit enum variant")
    }

    fn accept_tuple(&mut self, _visitor: &mut impl TupleRecv) -> anyhow::Result<()> {
        bail!("schema mismatch: expected unit enum variant")
    }

    fn accept_named(&mut self, _visitor: &mut impl TupleRecv) -> anyhow::Result<()> {
        bail!("schema mismatch: expected unit enum variant")
    }
}

pub(crate) fn encode_enum<W: WriteExt>(
    send: &impl EnumSend,
    writer: &mut TextWriter<W>,
    schema: &EnumSchema<'_>,
) -> anyhow::Result<()> {
    struct Accept<'writer, 'schema, W: WriteExt> {
        writer: &'writer mut TextWriter<W>,
        schema: &'schema EnumSchema<'schema>,
        active_indices: Vec<usize>,
    }

    impl<'writer, 'schema, W: WriteExt> EnumSendAccept for Accept<'writer, 'schema, W> {
        fn accept_unit(&mut self, index: isize) -> anyhow::Result<()> {
            let version = self.writer.version();
            let variant = variant_for_send(self.schema, &self.active_indices, index, version)?;
            match &variant.kind {
                VariantKind::Unit(_) => {
                    self.writer.start_object()?;
                    self.writer.next_in_object(true, variant.id)?;
                    self.writer.write_unit()?;
                    self.writer.finish_object()
                }
                _ => bail!("schema mismatch: expected unit enum variant"),
            }
        }

        fn accept_newtype(
            &mut self,
            index: isize,
            visitor: &impl NewTypeSend,
        ) -> anyhow::Result<()> {
            let version = self.writer.version();
            let variant = variant_for_send(self.schema, &self.active_indices, index, version)?;
            match &variant.kind {
                VariantKind::NewType(_) => {
                    write_variant_object(self.writer, variant.id, |writer| {
                        visitor.visit(&mut RecursiveSendVisitor::new(writer))
                    })
                }
                _ => bail!("schema mismatch: expected newtype enum variant"),
            }
        }

        fn accept_tuple(&mut self, index: isize, visitor: &impl TupleSend) -> anyhow::Result<()> {
            let version = self.writer.version();
            let variant = variant_for_send(self.schema, &self.active_indices, index, version)?;
            match &variant.kind {
                VariantKind::Tuple(tuple) => {
                    write_variant_object(self.writer, variant.id, |writer| {
                        encode_tuple(visitor, writer, tuple.elems.len())
                    })
                }
                _ => bail!("schema mismatch: expected tuple enum variant"),
            }
        }

        fn accept_named(&mut self, index: isize, visitor: &impl TupleSend) -> anyhow::Result<()> {
            let version = self.writer.version();
            let variant = variant_for_send(self.schema, &self.active_indices, index, version)?;
            match &variant.kind {
                VariantKind::Named(named) => {
                    write_variant_object(self.writer, variant.id, |writer| {
                        encode_named_struct(visitor, writer, named)
                    })
                }
                _ => bail!("schema mismatch: expected named enum variant"),
            }
        }
    }

    let active_indices = active_variant_indices(schema, writer.version())?;
    send.visit(&mut Accept {
        writer,
        schema,
        active_indices,
    })
}

pub(crate) fn decode_enum<R: ReadExt>(
    recv: &mut impl EnumRecv,
    reader: &mut TextReader<R>,
    schema: &EnumSchema<'_>,
) -> anyhow::Result<()> {
    struct Accept<'reader, 'schema, R: ReadExt> {
        reader: &'reader mut TextReader<R>,
        schema: &'schema EnumSchema<'schema>,
        variant_index: usize,
    }

    impl<'reader, 'schema, R: ReadExt> EnumRecvAccept for Accept<'reader, 'schema, R> {
        fn accept_newtype(&mut self, visitor: &mut impl NewTypeRecv) -> anyhow::Result<()> {
            match &self.schema.variants[self.variant_index].kind {
                VariantKind::NewType(_) => {
                    visitor.visit(&mut RecursiveRecvVisitor::new(self.reader))
                }
                _ => bail!("schema mismatch: expected newtype enum variant"),
            }
        }

        fn accept_tuple(&mut self, visitor: &mut impl TupleRecv) -> anyhow::Result<()> {
            match &self.schema.variants[self.variant_index].kind {
                VariantKind::Tuple(tuple) => decode_tuple(visitor, self.reader, tuple.elems.len()),
                _ => bail!("schema mismatch: expected tuple enum variant"),
            }
        }

        fn accept_named(&mut self, visitor: &mut impl TupleRecv) -> anyhow::Result<()> {
            match &self.schema.variants[self.variant_index].kind {
                VariantKind::Named(named) => decode_named_struct(visitor, self.reader, named),
                _ => bail!("schema mismatch: expected named enum variant"),
            }
        }
    }

    reader.start_object()?;
    let wire_id = match reader.next_in_object(true)? {
        Some(id) => id,
        None => bail!("enum is missing a variant id"),
    };

    let version = reader.version();
    let (variant_index, variant) = variant_for_recv(
        schema,
        &active_variant_indices(schema, version)?,
        wire_id,
        version,
    )?;

    match &variant.kind {
        VariantKind::Unit(_) => {
            reader.take_unit()?;
            recv.visit(variant_index as isize, &mut EmptyEnumRecvAccept)?;
        }
        VariantKind::NewType(_) | VariantKind::Tuple(_) | VariantKind::Named(_) => {
            recv.visit(
                variant_index as isize,
                &mut Accept {
                    reader,
                    schema,
                    variant_index,
                },
            )?;
        }
    }

    match reader.next_in_object(false)? {
        Some(actual) => bail!("enum has unexpected variant id {:x}", actual),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use kaloron::{TypeShape, Version};

    #[derive(Debug, PartialEq, TypeShape)]
    #[kaloron(introduced = "0.0.0")]
    enum ExampleEnum {
        #[kaloron(id = 0)]
        Unit,
        #[kaloron(id = 1)]
        NewType(u16),
        #[kaloron(id = 2, removed = "0.0.0.1")]
        Tuple(u8, u8),
        #[kaloron(id = 3)]
        Named {
            #[kaloron(id = 0)]
            x: u64,
            #[kaloron(id = 1)]
            y: String,
        },
    }

    fn assist_round_trip(value: ExampleEnum, version: Version, expected: &str) -> Result<()> {
        let out = super::super::assist_encode_test_value(version, &value)?;
        assert_eq!(String::from_utf8(out.clone())?, expected);

        let decoded: ExampleEnum = super::super::assist_decode_test_value(version, &out)?;
        assert_eq!(decoded, value);
        Ok(())
    }

    #[test]
    fn test_enum_round_trip_uses_single_entry_object_shape() -> Result<()> {
        assist_round_trip(ExampleEnum::Unit, Version::zero(), "{0:unit}")?;
        assist_round_trip(ExampleEnum::NewType(30), Version::zero(), "{1:u16:1e}")?;
        assist_round_trip(
            ExampleEnum::Tuple(1, 2),
            Version::zero(),
            "{2:[2$u8:1,u8:2]}",
        )?;
        assist_round_trip(
            ExampleEnum::Named {
                x: 1,
                y: "Ada".to_owned(),
            },
            Version::zero(),
            "{3:{0:u64:1,1:\"Ada\"}}",
        )?;
        Ok(())
    }

    #[test]
    fn test_enum_rejects_unknown_variant_id() {
        let err = super::super::assist_decode_test_value::<ExampleEnum>(Version::zero(), b"{6:unit}")
            .unwrap_err();
        assert!(err.to_string().contains("unknown enum variant id"));
    }

    #[test]
    fn test_enum_rejects_extra_object_fields() {
        let err =
            super::super::assist_decode_test_value::<ExampleEnum>(Version::zero(), b"{0:unit,2:u16:1e}")
                .unwrap_err();
        assert!(err.to_string().contains("unexpected variant id"));
    }

    #[test]
    fn test_enum_rejects_inactive_variant_at_version() {
        let version = Version::new(0, 0, 0, 1);

        assert!(super::super::assist_encode_test_value(version, &ExampleEnum::Tuple(1, 2)).is_err());
        assert!(
            super::super::assist_decode_test_value::<ExampleEnum>(version, b"{2:[2$u8:1,u8:2]}").is_err()
        );
    }
}
