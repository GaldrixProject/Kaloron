// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

mod named_struct_ir;
mod newtype_struct_ir;
mod tuple_struct_ir;
mod unit_struct_ir;

use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{DataStruct, DeriveInput};

use named_struct_ir::NamedStructIr;
use newtype_struct_ir::NewtypeStructIr;
use tuple_struct_ir::TupleStructIr;
use unit_struct_ir::UnitStructIr;

use crate::common::{
    parse_fields, validate_gated_fields, ContainerIr, FieldsOwner, ParsedFieldsIr,
};

enum StructKindIr {
    Unit(UnitStructIr),
    Newtype(NewtypeStructIr),
    Tuple(TupleStructIr),
    Named(NamedStructIr),
}

pub struct StructIr {
    container: ContainerIr,
    kind: StructKindIr,
}

impl StructIr {
    pub(crate) fn from(input: &DeriveInput, data: &DataStruct) -> syn::Result<Self> {
        let container = ContainerIr::from_input(input)?;
        let mut parsed = parse_fields(&data.fields, FieldsOwner::TupleStruct)?;

        // Validate and mark gated fields for named structs
        if let ParsedFieldsIr::Named(ref mut fields) = parsed {
            validate_gated_fields(fields, container.version())?;
        }

        let kind = StructKindIr::from_parsed(parsed);

        Ok(Self { container, kind })
    }

    pub(super) fn ident(&self) -> &Ident {
        self.container.ident()
    }

    pub(super) fn container_version(&self) -> &crate::version::VersionAttrs {
        self.container.version()
    }

    fn expand_schema(&self) -> TokenStream {
        match &self.kind {
            StructKindIr::Unit(ir) => ir.expand_schema_body(self),
            StructKindIr::Newtype(ir) => ir.expand_schema_body(self),
            StructKindIr::Tuple(ir) => ir.expand_schema_body(self),
            StructKindIr::Named(ir) => ir.expand_schema_body(self),
        }
    }

    fn expand_send(&self) -> TokenStream {
        match &self.kind {
            StructKindIr::Unit(ir) => ir.expand_send_body(),
            StructKindIr::Newtype(ir) => ir.expand_send_body(),
            StructKindIr::Tuple(ir) => ir.expand_send_body(),
            StructKindIr::Named(ir) => ir.expand_send_body(),
        }
    }

    fn expand_recv(&self) -> TokenStream {
        let (builder, value) = match &self.kind {
            StructKindIr::Unit(ir) => ir.expand_recv_value(),
            StructKindIr::Newtype(ir) => ir.expand_recv_value(),
            StructKindIr::Tuple(ir) => ir.expand_recv_value(),
            StructKindIr::Named(ir) => ir.expand_recv_value(),
        };
        quote! {
            #builder
            ::std::result::Result::Ok(Self #value)
        }
    }

    pub(crate) fn expand(&self) -> TokenStream {
        let marker_impl = match &self.kind {
            StructKindIr::Unit(_) => TokenStream::new(),
            StructKindIr::Newtype(_) => self.container.expand_marker_impl(quote! {
                ::kaloron::MarkerCompositeTypeShapeNewTypeStruct
            }),
            StructKindIr::Tuple(_) => self.container.expand_marker_impl(quote! {
                ::kaloron::MarkerCompositeTypeShapeTupleStruct
            }),
            StructKindIr::Named(_) => self.container.expand_marker_impl(quote! {
                ::kaloron::MarkerCompositeTypeShapeNamedStruct
            }),
        };

        let type_shape_impl = self.container.expand_type_shape_impl(
            self.expand_schema(),
            self.expand_send(),
            self.expand_recv(),
        );

        quote! {
            #marker_impl
            #type_shape_impl
        }
    }
}

impl StructKindIr {
    fn from_parsed(parsed: ParsedFieldsIr) -> Self {
        match parsed {
            ParsedFieldsIr::Unit => Self::Unit(UnitStructIr),
            ParsedFieldsIr::Newtype(ty) => Self::Newtype(NewtypeStructIr { ty }),
            ParsedFieldsIr::Tuple(fields) => Self::Tuple(TupleStructIr { fields }),
            ParsedFieldsIr::Named(fields) => Self::Named(NamedStructIr { fields }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::StructIr;
    use syn::DeriveInput;

    fn assist_parse_input(source: &str) -> DeriveInput {
        syn::parse_str(source).expect("valid derive input")
    }

    #[test]
    fn test_dispatches_unit_struct_codegen() {
        let input = assist_parse_input("#[kaloron(introduced = \"0.0.0\")] struct Unit;");
        let data = match &input.data {
            syn::Data::Struct(data) => data,
            _ => panic!("expected struct"),
        };

        let expanded = StructIr::from(&input, data).unwrap().expand().to_string();
        assert!(expanded.contains("accept_tuple_struct"));
    }

    #[test]
    fn test_dispatches_newtype_struct_codegen() {
        let input = assist_parse_input("#[kaloron(introduced = \"0.0.0\")] struct Newtype<T>(T);");
        let data = match &input.data {
            syn::Data::Struct(data) => data,
            _ => panic!("expected struct"),
        };

        let expanded = StructIr::from(&input, data).unwrap().expand().to_string();
        assert!(expanded.contains("accept_newtype_struct"));
        assert!(expanded.contains("MarkerCompositeTypeShapeNewTypeStruct"));
        assert!(expanded.contains("__Tb01"));
    }

    #[test]
    fn test_dispatches_tuple_struct_codegen() {
        let input = assist_parse_input("#[kaloron(introduced = \"0.0.0\")] struct Tuple(u32, String);");
        let data = match &input.data {
            syn::Data::Struct(data) => data,
            _ => panic!("expected struct"),
        };

        let expanded = StructIr::from(&input, data).unwrap().expand().to_string();
        assert!(expanded.contains("accept_tuple_struct"));
    }

    #[test]
    fn test_dispatches_named_struct_codegen() {
        let input = assist_parse_input(
            "#[kaloron(introduced = \"0.0.0\")] struct Named<T> { #[kaloron(id = 0)] value: T, #[kaloron(id = 1)] note: String }",
        );
        let data = match &input.data {
            syn::Data::Struct(data) => data,
            _ => panic!("expected struct"),
        };

        let expanded = StructIr::from(&input, data).unwrap().expand().to_string();
        assert!(expanded.contains("accept_named_struct"));
        assert!(expanded.contains("MarkerCompositeTypeShapeNamedStruct"));
        assert!(expanded.contains("NamedStructSchema"));
    }

    #[test]
    fn test_dispatches_large_tuple_struct_codegen() {
        let fields = std::iter::repeat_n("u8", 17).collect::<Vec<_>>().join(", ");
        let input = assist_parse_input(&format!(
            "#[kaloron(introduced = \"0.0.0\")] struct Big({fields});"
        ));
        let data = match &input.data {
            syn::Data::Struct(data) => data,
            _ => panic!("expected struct"),
        };

        let expanded = StructIr::from(&input, data).unwrap().expand().to_string();
        assert!(expanded.contains("__TbCp"));
        assert!(expanded.contains("__Tb0"));
        assert!(expanded.contains("__tbc"));
    }
}
