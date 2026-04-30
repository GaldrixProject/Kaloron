#![allow(clippy::type_complexity)]
// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use crate::utility::*;
use crate::*;
use seq_macro::seq;

fn tuple_bail() -> anyhow::Result<()> {
    anyhow::bail!("tuple index out of bounds")
}

// Unit type `()` is the zero-element tuple.  Its schema is `Schema::Tuple(&[])`.
impl TypeShape for () {
    const SCHEMA: &'static Schema<'static> = &Schema::Tuple(&[]);

    fn send(&self, _version: Version, send: &mut impl SendAccept) -> anyhow::Result<()> {
        send.accept_tuple(&__Tv0)
    }

    fn recv(_version: Version, recv: &mut impl RecvAccept) -> anyhow::Result<Self> {
        let mut builder = __Tb0;
        recv.accept_tuple(&mut builder)?;
        Ok(())
    }
}

seq!(N in 01..=32 {
    #(
        seq!(I in 0..N {
            impl< #(T~I: TypeShape,)* > TypeShape for ( #(T~I,)* ) {
                const SCHEMA: &'static Schema<'static> = { &Schema::Tuple(&[ #( T~I::SCHEMA, )* ]) };

                fn send(&self, _version: Version, send: &mut impl SendAccept) -> anyhow::Result<()> {
                    struct Send<'a, #(T~I,)*>(&'a ( #(T~I,)* ));
                    impl< #(T~I: TypeShape,)* > TupleSend for Send<'_, #(T~I,)*> {
                        fn visit(&self, index: isize, visitor: &mut impl SendVisitor) -> anyhow::Result<()> {
                            match index {
                                #( I => visitor.visit(&self.0.I), )*
                                _ => tuple_bail(),
                            }
                        }
                    }

                    send.accept_tuple(&Send(self))
                }

                fn recv(_version: Version, recv: &mut impl RecvAccept) -> anyhow::Result<Self> {
                    let mut builder: __Tb~N<#(T~I,)*> = Default::default();
                    recv.accept_tuple(&mut builder)?;
                    __tbc(&builder)?;
                    Ok(( #( builder.g~I(), )* ))
                }
            }
        });
    )*
});
