// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, Type};

pub(crate) struct RecvBuilderLayout {
    ty: TokenStream,
    field_accesses: Vec<TokenStream>,
}

impl RecvBuilderLayout {
    #[allow(dead_code)]
    pub(crate) fn builder_type(&self) -> &TokenStream {
        &self.ty
    }

    pub(crate) fn expand_recv(&self, accept_type: &str) -> TokenStream {
        let builder_ty = &self.ty;
        let accept_fn = format_ident!("accept_{}", accept_type);

        quote! {
            let mut builder: #builder_ty = ::std::default::Default::default();
            recv.#accept_fn(&mut builder)?;
            ::kaloron::__tbc(&builder)?;
        }
    }

    /// Generate builder declaration + accept call, without the completeness check.
    pub(crate) fn expand_recv_start(&self, accept_type: &str) -> TokenStream {
        let builder_ty = &self.ty;
        let accept_fn = format_ident!("accept_{}", accept_type);

        quote! {
            let mut builder: #builder_ty = ::std::default::Default::default();
            recv.#accept_fn(&mut builder)?;
        }
    }

    /// Generate the completeness check.
    pub(crate) fn expand_recv_check(&self) -> TokenStream {
        quote! {
            ::kaloron::__tbc(&builder)?;
        }
    }

    pub(crate) fn expand_named_value(&self, field_names: &[&Ident]) -> TokenStream {
        let field_takes =
            field_names
                .iter()
                .zip(self.field_accesses.iter())
                .map(|(field_name, access)| {
                    quote! { #field_name: builder #access }
                });

        quote! {
            { #( #field_takes, )* }
        }
    }

    pub(crate) fn expand_unnamed_value(&self) -> TokenStream {
        let field_takes = self.field_accesses.iter().map(|access| {
            quote! { builder #access }
        });

        quote! {
            ( #( #field_takes, )* )
        }
    }
}

struct RecvBuilderNode {
    ty: TokenStream,
    field_accesses: Vec<TokenStream>,
}

fn make_leaf_node(field_tys: &[&Type]) -> RecvBuilderNode {
    let builder_ident = format_ident!("__Tb{:02}", field_tys.len());
    let chunk_types = field_tys.iter();
    let field_accesses = (0..field_tys.len())
        .map(|index| {
            let getter = format_ident!("g{}", index);
            quote! { .#getter() }
        })
        .collect();

    RecvBuilderNode {
        ty: quote! { ::kaloron::#builder_ident<#( #chunk_types, )*> },
        field_accesses,
    }
}

fn rollup_nodes(nodes: Vec<RecvBuilderNode>) -> Vec<RecvBuilderNode> {
    nodes
        .chunks(16)
        .map(|chunk| {
            let child_tys = chunk.iter().map(|node| &node.ty);
            let pad_count = 16 - chunk.len();
            let pad_types = std::iter::repeat_with(|| quote! { ::kaloron::__Tb0 }).take(pad_count);

            let field_accesses = chunk
                .iter()
                .enumerate()
                .flat_map(|(child_index, node)| {
                    let child_index = syn::Index::from(child_index);
                    node.field_accesses.iter().map(move |suffix| {
                        quote! { .#child_index #suffix }
                    })
                })
                .collect();

            RecvBuilderNode {
                ty: quote! { ::kaloron::__TbCp<#( #child_tys, )* #( #pad_types, )*> },
                field_accesses,
            }
        })
        .collect()
}

pub(crate) fn recv_builder_layout(field_tys: &[&Type]) -> RecvBuilderLayout {
    if field_tys.is_empty() {
        return RecvBuilderLayout {
            ty: quote! { ::kaloron::__Tb0 },
            field_accesses: Vec::new(),
        };
    }

    let mut nodes: Vec<RecvBuilderNode> = field_tys.chunks(16).map(make_leaf_node).collect();

    while nodes.len() > 1 {
        nodes = rollup_nodes(nodes);
    }

    let root = nodes.pop().expect("non-empty recv builder layout");
    RecvBuilderLayout {
        ty: root.ty,
        field_accesses: root.field_accesses,
    }
}

#[cfg(test)]
mod tests {
    use super::recv_builder_layout;
    use quote::quote;
    use syn::{Ident, Type};

    fn assist_parse_types(source: &[&str]) -> Vec<Type> {
        source
            .iter()
            .map(|source| syn::parse_str::<Type>(source).expect("valid type"))
            .collect()
    }

    #[test]
    fn test_layouts_empty_fields_as_tb0() {
        let layout = recv_builder_layout(&[]);
        assert_eq!(
            layout.ty.to_string(),
            quote! { ::kaloron::__Tb0 }.to_string()
        );
        assert!(layout.field_accesses.is_empty());
    }

    #[test]
    fn test_layouts_single_chunk_with_direct_getters() {
        let tys = assist_parse_types(&["u8", "u16", "u32"]);
        let refs = tys.iter().collect::<Vec<_>>();
        let layout = recv_builder_layout(&refs);

        assert_eq!(
            layout.ty.to_string(),
            quote! { ::kaloron::__Tb03<u8, u16, u32,> }.to_string()
        );
        assert_eq!(layout.field_accesses.len(), 3);
        assert_eq!(
            layout.field_accesses[0].to_string(),
            quote! { .g0() }.to_string()
        );
        assert_eq!(
            layout.field_accesses[1].to_string(),
            quote! { .g1() }.to_string()
        );
        assert_eq!(
            layout.field_accesses[2].to_string(),
            quote! { .g2() }.to_string()
        );
    }

    #[test]
    fn test_layouts_multi_chunk_with_nested_prefixes() {
        let tys = (0..17)
            .map(|_| syn::parse_str::<Type>("u8").unwrap())
            .collect::<Vec<_>>();
        let refs = tys.iter().collect::<Vec<_>>();
        let layout = recv_builder_layout(&refs);

        assert!(layout.ty.to_string().contains("__TbCp"));
        assert_eq!(layout.field_accesses.len(), 17);
        assert_eq!(
            layout.field_accesses[0].to_string(),
            quote! { .0.g0() }.to_string()
        );
        assert_eq!(
            layout.field_accesses[15].to_string(),
            quote! { .0.g15() }.to_string()
        );
        assert_eq!(
            layout.field_accesses[16].to_string(),
            quote! { .1.g0() }.to_string()
        );
    }

    #[test]
    fn test_layouts_multi_level_prefixes_iteratively() {
        let tys = (0..257)
            .map(|_| syn::parse_str::<Type>("u8").unwrap())
            .collect::<Vec<_>>();
        let refs = tys.iter().collect::<Vec<_>>();
        let layout = recv_builder_layout(&refs);

        assert!(layout.ty.to_string().contains("__TbCp"));
        assert_eq!(layout.field_accesses.len(), 257);
        assert_eq!(
            layout.field_accesses[0].to_string(),
            quote! { .0 .0 .g0() }.to_string()
        );
        assert_eq!(
            layout.field_accesses[255].to_string(),
            quote! { .0 .15 .g15() }.to_string()
        );
        assert_eq!(
            layout.field_accesses[256].to_string(),
            quote! { .1 .0 .g0() }.to_string()
        );
    }

    #[test]
    fn test_expands_named_and_unnamed_recv_values() {
        let tys = assist_parse_types(&["u8", "u16"]);
        let refs = tys.iter().collect::<Vec<_>>();
        let layout = recv_builder_layout(&refs);

        let names = [
            syn::parse_str::<Ident>("first").unwrap(),
            syn::parse_str::<Ident>("second").unwrap(),
        ];
        let name_refs = names.iter().collect::<Vec<_>>();

        assert_eq!(
            layout.expand_named_value(&name_refs).to_string(),
            quote! { { first: builder .g0(), second: builder .g1(), } }.to_string()
        );
        assert_eq!(
            layout.expand_unnamed_value().to_string(),
            quote! { ( builder .g0(), builder .g1(), ) }.to_string()
        );
    }
}
