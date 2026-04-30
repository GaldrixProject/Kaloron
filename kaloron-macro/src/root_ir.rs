// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use crate::enum_ir::EnumIr;
use crate::struct_ir::StructIr;
use proc_macro2::TokenStream;
use syn::{Data, DeriveInput};

pub(crate) enum RootIr {
    Struct(StructIr),
    Enum(EnumIr),
}

impl RootIr {
    pub(crate) fn from_input(input: &DeriveInput) -> syn::Result<Self> {
        match &input.data {
            Data::Struct(data_struct) => Ok(RootIr::Struct(StructIr::from(input, data_struct)?)),
            Data::Enum(data_enum) => Ok(RootIr::Enum(EnumIr::from(input, data_enum)?)),
            Data::Union(_) => Err(syn::Error::new_spanned(
                input,
                "derive(TypeShape) does not support unions",
            )),
        }
    }

    pub(crate) fn expand(&self) -> TokenStream {
        match self {
            RootIr::Struct(ir) => ir.expand(),
            RootIr::Enum(ir) => ir.expand(),
        }
    }
}
