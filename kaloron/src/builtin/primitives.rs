// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use crate::*;

#[cfg(unix)]
use std::os::fd::OwnedFd;

macro_rules! impl_primitive_shape {
    ($t:ty, $schema:expr, $method:ident) => {
        impl TypeShape for $t {
            const SCHEMA: &'static Schema<'static> = {
                const VALUE: Schema<'static> = $schema;
                &VALUE
            };

            fn send(&self, _version: Version, send: &mut impl SendAccept) -> anyhow::Result<()> {
                struct Send<'a>(&'a $t);

                impl PrimitiveSend for Send<'_> {
                    fn visit(&self, visitor: &mut impl PrimitiveSendVisitor) -> anyhow::Result<()> {
                        visitor.$method(&self.0)
                    }
                }

                send.accept_primitive(&Send(self))
            }

            fn recv(_version: Version, recv: &mut impl RecvAccept) -> anyhow::Result<Self> {
                #[allow(non_camel_case_types)]
                struct Recv {
                    value: Option<$t>,
                }

                impl PrimitiveRecv for Recv {
                    fn visit(
                        &mut self,
                        visitor: &mut impl PrimitiveRecvVisitor,
                    ) -> anyhow::Result<()> {
                        self.value = Some(visitor.$method()?);
                        Ok(())
                    }
                }

                let mut helper = Recv { value: None };
                recv.accept_primitive(&mut helper)?;
                match helper.value {
                    Some(value) => Ok(value),
                    None => Err(anyhow::anyhow!("primitive value was not received")),
                }
            }
        }
    };
}

impl_primitive_shape!(bool, Schema::Primitive(Primitive::Bool), visit_bool);
impl_primitive_shape!(i8, Schema::Primitive(Primitive::I8), visit_i8);
impl_primitive_shape!(i16, Schema::Primitive(Primitive::I16), visit_i16);
impl_primitive_shape!(i32, Schema::Primitive(Primitive::I32), visit_i32);
impl_primitive_shape!(i64, Schema::Primitive(Primitive::I64), visit_i64);
impl_primitive_shape!(i128, Schema::Primitive(Primitive::I128), visit_i128);
impl_primitive_shape!(u8, Schema::Primitive(Primitive::U8), visit_u8);
impl_primitive_shape!(u16, Schema::Primitive(Primitive::U16), visit_u16);
impl_primitive_shape!(u32, Schema::Primitive(Primitive::U32), visit_u32);
impl_primitive_shape!(u64, Schema::Primitive(Primitive::U64), visit_u64);
impl_primitive_shape!(u128, Schema::Primitive(Primitive::U128), visit_u128);
impl_primitive_shape!(f32, Schema::Primitive(Primitive::F32), visit_f32);
impl_primitive_shape!(f64, Schema::Primitive(Primitive::F64), visit_f64);
impl_primitive_shape!(char, Schema::Primitive(Primitive::Char), visit_char);
impl_primitive_shape!(String, Schema::Primitive(Primitive::String), visit_string);

#[cfg(unix)]
impl TypeShape for OwnedFd {
    const SCHEMA: &'static Schema<'static> = &Schema::Primitive(Primitive::File);

    fn send(&self, _version: Version, send: &mut impl SendAccept) -> anyhow::Result<()> {
        struct Send<'a>(&'a OwnedFd);

        impl PrimitiveSend for Send<'_> {
            fn visit(&self, visitor: &mut impl PrimitiveSendVisitor) -> anyhow::Result<()> {
                visitor.visit_file(self.0)
            }
        }

        send.accept_primitive(&Send(self))
    }

    fn recv(_version: Version, recv: &mut impl RecvAccept) -> anyhow::Result<Self> {
        #[allow(non_camel_case_types)]
        struct Recv {
            value: Option<OwnedFd>,
        }

        impl PrimitiveRecv for Recv {
            fn visit(&mut self, visitor: &mut impl PrimitiveRecvVisitor) -> anyhow::Result<()> {
                self.value = Some(visitor.visit_file()?);
                Ok(())
            }
        }

        let mut helper = Recv { value: None };
        recv.accept_primitive(&mut helper)?;
        match helper.value {
            Some(value) => Ok(value),
            None => Err(anyhow::anyhow!("primitive value was not received")),
        }
    }
}
