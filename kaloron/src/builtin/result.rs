// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use crate::*;

#[inline(never)]
fn result_ok_payload_missing() -> anyhow::Result<()> {
    Err(anyhow::anyhow!("Result::Ok payload not received"))
}

#[inline(never)]
fn result_err_payload_missing() -> anyhow::Result<()> {
    Err(anyhow::anyhow!("Result::Err payload not received"))
}

#[inline(never)]
fn unknown_result_variant_index(index: isize) -> anyhow::Result<()> {
    Err(anyhow::anyhow!("unknown Result variant index: {}", index))
}

#[inline(never)]
fn result_value_missing() -> anyhow::Error {
    anyhow::anyhow!("Result value not received")
}

struct OkSend<'a, V>(&'a V);

impl<'a, V: TypeShape> NewTypeSend for OkSend<'a, V> {
    fn visit(&self, visitor: &mut impl SendVisitor) -> anyhow::Result<()> {
        visitor.visit(self.0)
    }
}

struct ErrSend<'a, R>(&'a R);

impl<'a, R: TypeShape> NewTypeSend for ErrSend<'a, R> {
    fn visit(&self, visitor: &mut impl SendVisitor) -> anyhow::Result<()> {
        visitor.visit(self.0)
    }
}

struct Send<'a, V, R>(&'a Result<V, R>);

impl<'a, V: TypeShape, R: TypeShape> EnumSend for Send<'a, V, R> {
    fn visit(&self, accept: &mut impl EnumSendAccept) -> anyhow::Result<()> {
        match self.0 {
            Ok(value) => accept.accept_newtype(0, &OkSend(value)),
            Err(error) => accept.accept_newtype(1, &ErrSend(error)),
        }
    }
}

struct OkRecv<V> {
    value: Option<V>,
}

impl<V: TypeShape> NewTypeRecv for OkRecv<V> {
    fn visit(&mut self, visitor: &mut impl RecvVisitor) -> anyhow::Result<()> {
        self.value = Some(visitor.visit::<V>()?);
        Ok(())
    }
}

struct ErrRecv<R> {
    value: Option<R>,
}
impl<R: TypeShape> NewTypeRecv for ErrRecv<R> {
    fn visit(&mut self, visitor: &mut impl RecvVisitor) -> anyhow::Result<()> {
        self.value = Some(visitor.visit::<R>()?);
        Ok(())
    }
}

struct Recv<V, R> {
    value: Option<Result<V, R>>,
}

impl<V: TypeShape, R: TypeShape> EnumRecv for Recv<V, R> {
    fn visit(&mut self, index: isize, accept: &mut impl EnumRecvAccept) -> anyhow::Result<()> {
        match index {
            0 => {
                let mut ok_recv: OkRecv<V> = OkRecv { value: None };
                accept.accept_newtype(&mut ok_recv)?;
                match ok_recv.value {
                    Some(v) => {
                        self.value = Some(Ok(v));
                        Ok(())
                    }
                    None => result_ok_payload_missing(),
                }
            }
            1 => {
                let mut err_recv: ErrRecv<R> = ErrRecv { value: None };
                accept.accept_newtype(&mut err_recv)?;
                match err_recv.value {
                    Some(e) => {
                        self.value = Some(Err(e));
                        Ok(())
                    }
                    None => result_err_payload_missing(),
                }
            }
            _ => unknown_result_variant_index(index),
        }
    }
}

impl<T: TypeShape, E: TypeShape> TypeShape for Result<T, E> {
    const SCHEMA: &'static Schema<'static> = &Schema::Enum(EnumSchema::new(
        "Result",
        &[
            Variant::newtype(0, "Ok", T::SCHEMA, VersionRange::default()),
            Variant::newtype(1, "Err", E::SCHEMA, VersionRange::default()),
        ],
        VersionRange::default(),
        &[],
    ));

    fn send(&self, _version: Version, send: &mut impl SendAccept) -> anyhow::Result<()> {
        send.accept_enum(&Send(self))
    }

    fn recv(_version: Version, recv: &mut impl RecvAccept) -> anyhow::Result<Self> {
        let mut builder: Recv<T, E> = Recv { value: None };
        recv.accept_enum(&mut builder)?;
        match builder.value {
            Some(v) => Ok(v),
            None => Err(result_value_missing()),
        }
    }
}
