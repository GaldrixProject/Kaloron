// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use crate::version::{parse_kaloron_attrs, validate_version_spec, VersionAttrs};
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{parse_quote, DeriveInput, GenericParam, Generics};

#[derive(Clone)]
pub(crate) struct ContainerIr {
    ident: Ident,
    generics: Generics,
    version: VersionAttrs,
}

impl ContainerIr {
    pub(crate) fn from_input(input: &DeriveInput) -> syn::Result<Self> {
        let version = parse_kaloron_attrs(&input.attrs)?;
        if version.introduced.is_none() {
            return Err(syn::Error::new_spanned(
                input,
                format!(
                    "derive(TypeShape) requires #[kaloron(introduced = \"X.Y.Z\")] on {}",
                    input.ident
                ),
            ));
        }
        validate_version_spec(&version, input.ident.span())?;
        Ok(Self {
            ident: input.ident.clone(),
            generics: input.generics.clone(),
            version,
        })
    }
    pub(crate) fn ident(&self) -> &Ident {
        &self.ident
    }
    pub(crate) fn generics(&self) -> &Generics {
        &self.generics
    }
    pub(crate) fn version(&self) -> &VersionAttrs {
        &self.version
    }
    pub(crate) fn bounded_generics(&self) -> Generics {
        let mut generics = self.generics.clone();
        for param in &mut generics.params {
            if let GenericParam::Type(type_param) = param {
                type_param.bounds.push(parse_quote!(::kaloron::TypeShape));
            }
        }
        generics
    }

    pub(crate) fn helper_generics(&self) -> Generics {
        let mut generics = self.generics.clone();
        generics.params.insert(0, parse_quote!('a));
        generics
    }

    pub(crate) fn expand_marker_impl(&self, marker_trait: TokenStream) -> TokenStream {
        let name = &self.ident;
        let generics = self.generics.clone();
        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
        quote! {
            impl #impl_generics #marker_trait for #name #ty_generics #where_clause {}
        }
    }

    pub(crate) fn expand_type_shape_impl(
        &self,
        schema_body: TokenStream,
        send_body: TokenStream,
        recv_body: TokenStream,
    ) -> TokenStream {
        let name = &self.ident;
        let generics = self.bounded_generics();
        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
        quote! {
            #[allow(unused_variables)]
            impl #impl_generics ::kaloron::TypeShape for #name #ty_generics #where_clause {
                const SCHEMA: &'static ::kaloron::Schema<'static> = {
                    #schema_body
                };

                fn send(&self, version: ::kaloron::Version, send: &mut impl ::kaloron::SendAccept) -> ::anyhow::Result<()> {
                    #send_body
                }

                fn recv(version: ::kaloron::Version, recv: &mut impl ::kaloron::RecvAccept) -> ::anyhow::Result<Self> {
                    #recv_body
                }
            }
        }
    }
}
