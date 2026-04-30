// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::{Ident, Type};

pub(crate) struct SendBuilderLayout {
    value: TokenStream,
    field_count: usize,
}

impl SendBuilderLayout {
    /// Get the raw value expression (the constructed `__Tv` tree).
    /// Field bindings `field_0`, `field_1`, … must be in scope when this is used.
    pub(crate) fn value_expr(&self) -> &TokenStream {
        &self.value
    }

    pub(crate) fn expand_unnamed_bind(&self) -> TokenStream {
        let field_bindings =
            (0..self.field_count).map(|field_index| format_ident!("field_{}", field_index));

        quote! { ( #( #field_bindings ),* ) }
    }

    pub(crate) fn expand_named_bind(&self, field_names: &[&Ident]) -> TokenStream {
        assert_eq!(
            field_names.len(),
            self.field_count,
            "send builder named binding count did not match field layout",
        );

        let field_bindings = field_names
            .iter()
            .enumerate()
            .map(|(field_index, field_name)| {
                let field_binding = format_ident!("field_{}", field_index);
                quote! { #field_name: #field_binding }
            });

        quote! { { #( #field_bindings ),* } }
    }

    pub(crate) fn expand_send(&self, accept_type: &str, index: &impl ToTokens) -> TokenStream {
        let accept_fn = format_ident!("accept_{}", accept_type);
        let value = &self.value;
        quote! { send.#accept_fn(#index, &#value) }
    }
}

fn make_leaf_value(field_tys: &[&Type], field_offset: usize) -> TokenStream {
    let builder_ident = format_ident!("__Tv{:02}", field_tys.len());
    let values = (0..field_tys.len()).map(|field_index| {
        let value = format_ident!("field_{}", field_offset + field_index);
        quote! { #value }
    });

    quote! {
        ::kaloron::#builder_ident( #( #values, )* )
    }
}

fn rollup_values(values: Vec<TokenStream>) -> Vec<TokenStream> {
    values
        .chunks(16)
        .map(|chunk| {
            let child_values = chunk.iter();
            let pad_count = 16 - chunk.len();
            let pad_values = std::iter::repeat_with(|| quote! { ::kaloron::__Tv0 }).take(pad_count);

            quote! {
                ::kaloron::__TvCp( #( #child_values, )* #( #pad_values, )* )
            }
        })
        .collect()
}

pub(crate) fn send_builder_layout(field_tys: &[&Type]) -> SendBuilderLayout {
    if field_tys.is_empty() {
        return SendBuilderLayout {
            value: quote! { ::kaloron::__Tv0 },
            field_count: 0,
        };
    }

    let mut values: Vec<TokenStream> = field_tys
        .chunks(16)
        .enumerate()
        .map(|(chunk_index, chunk)| make_leaf_value(chunk, chunk_index * 16))
        .collect();

    while values.len() > 1 {
        values = rollup_values(values);
    }

    SendBuilderLayout {
        value: values.pop().expect("non-empty send builder layout"),
        field_count: field_tys.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::send_builder_layout;
    use quote::quote;
    use syn::{Ident, Type};

    fn assist_parse_types(source: &[&str]) -> Vec<Type> {
        source
            .iter()
            .map(|source| syn::parse_str::<Type>(source).expect("valid type"))
            .collect()
    }

    fn assist_parse_idents(source: &[&str]) -> Vec<Ident> {
        source
            .iter()
            .map(|source| syn::parse_str::<Ident>(source).expect("valid ident"))
            .collect()
    }

    #[test]
    fn test_layouts_empty_fields_as_tv0() {
        let layout = send_builder_layout(&[]);

        assert_eq!(
            layout.expand_unnamed_bind().to_string(),
            quote! { () }.to_string()
        );
        assert_eq!(
            layout.expand_send("tuple", &quote! { idx }).to_string(),
            quote! { send.accept_tuple(idx, &::kaloron::__Tv0) }.to_string()
        );
    }

    #[test]
    fn test_layouts_single_chunk_with_direct_tuple_value() {
        let tys = assist_parse_types(&["u8", "u16", "u32"]);
        let refs = tys.iter().collect::<Vec<_>>();
        let layout = send_builder_layout(&refs);

        assert_eq!(
            layout.expand_unnamed_bind().to_string(),
            quote! { (field_0, field_1, field_2) }.to_string()
        );
        assert_eq!(
            layout.expand_send("tuple", &quote! { idx }).to_string(),
            quote! { send.accept_tuple(idx, &::kaloron::__Tv03(field_0, field_1, field_2, )) }
                .to_string()
        );
    }

    #[test]
    fn test_layouts_named_binding_pattern_from_field_names() {
        let tys = assist_parse_types(&["u8", "u16"]);
        let refs = tys.iter().collect::<Vec<_>>();
        let layout = send_builder_layout(&refs);
        let names = assist_parse_idents(&["first", "second"]);
        let name_refs = names.iter().collect::<Vec<_>>();

        assert_eq!(
            layout.expand_named_bind(&name_refs).to_string(),
            quote! { { first: field_0, second: field_1 } }.to_string()
        );
    }

    #[test]
    fn test_layouts_multi_chunk_with_nested_value_tree() {
        let tys = (0..17)
            .map(|_| syn::parse_str::<Type>("u8").unwrap())
            .collect::<Vec<_>>();
        let refs = tys.iter().collect::<Vec<_>>();
        let layout = send_builder_layout(&refs);

        let value = layout.expand_send("tuple", &quote! { idx }).to_string();
        assert!(value.contains("accept_tuple"));
        assert!(value.contains("idx"));
        assert!(value.contains("__TvCp"));
        assert!(value.contains("__Tv16"));
        assert!(value.contains("__Tv01"));
        assert!(value.contains("field_0"));
        assert!(value.contains("field_15"));
        assert!(value.contains("field_16"));
    }

    #[test]
    fn test_layouts_multi_level_trees_iteratively() {
        let tys = (0..257)
            .map(|_| syn::parse_str::<Type>("u8").unwrap())
            .collect::<Vec<_>>();
        let refs = tys.iter().collect::<Vec<_>>();
        let layout = send_builder_layout(&refs);

        let value = layout.expand_send("tuple", &quote! { idx }).to_string();
        assert!(value.matches("__TvCp").count() >= 2);
        assert!(value.contains("accept_tuple"));
        assert!(value.contains("idx"));
        assert!(value.contains("field_0"));
        assert!(value.contains("field_255"));
        assert!(value.contains("field_256"));
    }

    #[test]
    fn test_expands_indexed_accept_calls_from_generated_bindings() {
        let tys = assist_parse_types(&["u8", "u16"]);
        let refs = tys.iter().collect::<Vec<_>>();
        let layout = send_builder_layout(&refs);

        assert_eq!(
            layout.expand_send("tuple", &quote! { idx }).to_string(),
            quote! { send.accept_tuple(idx, &::kaloron::__Tv02(field_0, field_1, )) }.to_string()
        );
    }
}
