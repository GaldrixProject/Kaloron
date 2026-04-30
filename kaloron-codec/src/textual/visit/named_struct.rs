// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use super::super::{TextReader, TextWriter};
use super::common::*;
use crate::{ReadExt, WriteExt};
use anyhow::bail;
use kaloron::*;

fn active_field_indices(
    schema: &NamedStructSchema<'_>,
    version: Version,
) -> anyhow::Result<Vec<usize>> {
    match schema.active_fields_at(version) {
        Some(states) => {
            if states.len() != schema.fields.len() {
                bail!(
                    "schema mismatch: named struct activation table length does not match field count"
                );
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
        None => Ok((0..schema.fields.len()).collect()),
    }
}

pub(crate) fn encode_named_struct<W: WriteExt>(
    send: &impl TupleSend,
    writer: &mut TextWriter<W>,
    schema: &NamedStructSchema<'_>,
) -> anyhow::Result<()> {
    let field_indices = active_field_indices(schema, writer.version())?;

    writer.start_object()?;
    let mut first = true;
    for index in field_indices {
        let field = &schema.fields[index];
        writer.next_in_object(first, field.id)?;
        first = false;

        send.visit(index as isize, &mut RecursiveSendVisitor::new(writer))?;
    }
    writer.finish_object()
}

pub(crate) fn decode_named_struct<R: ReadExt>(
    recv: &mut impl TupleRecv,
    reader: &mut TextReader<R>,
    schema: &NamedStructSchema<'_>,
) -> anyhow::Result<()> {
    let field_indices = active_field_indices(schema, reader.version())?;

    reader.start_object()?;
    let mut first = true;
    for index in field_indices {
        let field = &schema.fields[index];
        let actual = reader.next_in_object(first)?;
        first = false;

        match actual {
            Some(got) if got == field.id => {}
            Some(got) => {
                bail!(
                    "named struct field order mismatch: expected id {:x}, got {:x}",
                    field.id,
                    got
                )
            }
            None => bail!("named struct is missing field id {:x}", field.id),
        }

        recv.visit(index as isize, &mut RecursiveRecvVisitor::new(reader))?;
    }

    match reader.next_in_object(first)? {
        Some(actual) => bail!("named struct has unexpected field id {:x}", actual),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use kaloron::{TypeShape, Version};

    #[derive(Debug, PartialEq, TypeShape)]
    #[kaloron(introduced = "0.0.0")]
    struct ExampleNamedStruct {
        #[kaloron(id = 0)]
        a: u8,
        #[kaloron(id = 1)]
        b: u8,
    }

    #[test]
    fn test_named_struct_round_trip_uses_object_shape() -> Result<()> {
        let value = ExampleNamedStruct { a: 1, b: 2 };
        let out = super::super::assist_encode_test_value(Version::zero(), &value)?;
        assert_eq!(String::from_utf8(out.clone())?, "{0:u8:1,1:u8:2}");

        let decoded: ExampleNamedStruct = super::super::assist_decode_test_value(Version::zero(), &out)?;
        assert_eq!(decoded, value);
        Ok(())
    }

    #[test]
    fn test_named_struct_rejects_out_of_order_fields() {
        let err = super::super::assist_decode_test_value::<ExampleNamedStruct>(
            Version::zero(),
            b"{1:u8:2,0:u8:1}",
        )
        .unwrap_err();
        assert!(err.to_string().contains("field order mismatch"));
    }
}
