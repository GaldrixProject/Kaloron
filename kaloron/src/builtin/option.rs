// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use crate::*;

impl<T: TypeShape> TypeShape for Option<T> {
    const SCHEMA: &'static Schema<'static> = &Schema::Option(T::SCHEMA);

    fn send(&self, _version: Version, send: &mut impl SendAccept) -> anyhow::Result<()> {
        struct Send<'a, T>(&'a Option<T>);

        impl<T: TypeShape> OptionSend for Send<'_, T> {
            fn is_some(&self) -> bool {
                self.0.is_some()
            }

            fn visit(&self, visitor: &mut impl SendVisitor) -> anyhow::Result<()> {
                match self.0 {
                    Some(value) => visitor.visit(value),
                    None => Ok(()),
                }
            }
        }

        send.accept_option(&Send(self))
    }

    fn recv(_version: Version, recv: &mut impl RecvAccept) -> anyhow::Result<Self> {
        #[allow(non_camel_case_types)]
        struct Recv<T> {
            is_some: bool,
            value: Option<T>,
        }

        impl<T: TypeShape> OptionRecv for Recv<T> {
            fn as_some(&mut self, some: bool) -> anyhow::Result<()> {
                self.is_some = some;
                if !some {
                    self.value = None;
                }
                Ok(())
            }

            fn visit(&mut self, visitor: &mut impl RecvVisitor) -> anyhow::Result<()> {
                self.value = Some(visitor.visit::<T>()?);
                Ok(())
            }
        }

        let mut helper: Recv<T> = Recv {
            is_some: false,
            value: None,
        };
        recv.accept_option(&mut helper)?;
        if helper.is_some {
            match helper.value {
                Some(value) => Ok(Some(value)),
                None => Err(anyhow::anyhow!(
                    "option was signaled as Some but payload was not received"
                )),
            }
        } else {
            Ok(None)
        }
    }
}
