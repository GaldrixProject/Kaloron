// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

extern crate proc_macro;

use proc_macro::TokenStream;
use root_ir::RootIr;
use syn::{parse_macro_input, DeriveInput};

mod common;
mod enum_ir;
mod root_ir;
mod struct_ir;
mod version;

#[proc_macro_derive(TypeShape, attributes(kaloron))]
pub fn derive_type_shape(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match RootIr::from_input(&input) {
        Ok(ir) => ir.expand().into(),
        Err(err) => err.to_compile_error().into(),
    }
}

#[cfg(test)]
mod tests {
    use super::root_ir::RootIr;
    use syn::DeriveInput;

    fn assist_parse_input(source: &str) -> DeriveInput {
        syn::parse_str(source).expect("valid derive input")
    }

    #[test]
    fn test_rejects_tuple_structs_over_32_fields() {
        let fields = std::iter::repeat_n("u8", 33).collect::<Vec<_>>().join(", ");
        let input = assist_parse_input(&format!(
            "#[kaloron(introduced = \"0.0.0\")] struct Big({fields});"
        ));

        let err = match RootIr::from_input(&input) {
            Ok(_) => panic!("expected tuple struct limit error"),
            Err(err) => err,
        };
        assert!(err
            .to_string()
            .contains("supports tuple structs with at most 32 fields"));
    }

    #[test]
    fn test_rejects_tuple_variants_over_32_fields() {
        let fields = std::iter::repeat_n("u8", 33).collect::<Vec<_>>().join(", ");
        let input = assist_parse_input(&format!(
            "#[kaloron(introduced = \"0.0.0\")] enum Big {{ #[kaloron(id = 0)] V({fields}) }}"
        ));

        let err = match RootIr::from_input(&input) {
            Ok(_) => panic!("expected tuple variant limit error"),
            Err(err) => err,
        };
        assert!(err
            .to_string()
            .contains("supports tuple variants with at most 32 fields"));
    }
}
