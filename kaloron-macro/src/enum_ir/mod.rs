// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

mod named_variant_ir;
mod newtype_variant_ir;
mod tuple_variant_ir;
mod unit_variant_ir;

use proc_macro2::{Ident, Literal, TokenStream};
use quote::quote;
use syn::{DataEnum, DeriveInput};

use named_variant_ir::NamedVariantIr;
use newtype_variant_ir::NewtypeVariantIr;
use tuple_variant_ir::TupleVariantIr;
use unit_variant_ir::UnitVariantIr;

use crate::common::{
    parse_fields, validate_gated_fields, ContainerIr, FieldsOwner, ParsedFieldsIr,
};
use crate::version::{
    compute_activation_table_tokens, parse_kaloron_attrs, validate_dense_ids, VersionAttrs,
};

#[derive(Clone)]
enum VariantFieldsIr {
    Unit(UnitVariantIr),
    Newtype(NewtypeVariantIr),
    Tuple(TupleVariantIr),
    Named(NamedVariantIr),
}

#[derive(Clone)]
pub(crate) struct VariantIr {
    id: u32,
    ident: Ident,
    name: String,
    fields: VariantFieldsIr,
    pub(crate) version: VersionAttrs,
}

pub struct EnumIr {
    container: ContainerIr,
    variants: Vec<VariantIr>,
}

impl EnumIr {
    pub(crate) fn from(input: &DeriveInput, data: &DataEnum) -> syn::Result<Self> {
        let container = ContainerIr::from_input(input)?;
        let container_version = container.version().clone();

        let variants = data
            .variants
            .iter()
            .map(|variant| {
                let ident = variant.ident.clone();
                let raw_version = parse_kaloron_attrs(&variant.attrs)?;
                let id = raw_version.id.ok_or_else(|| {
                    syn::Error::new_spanned(
                        variant,
                        "enum variants require a #[kaloron(id = N)] attribute",
                    )
                })?;
                let inherited_version = raw_version.inherit_from(&container_version);
                let mut parsed = parse_fields(&variant.fields, FieldsOwner::TupleVariant)?;

                // Validate and mark gated fields for named variants
                if let ParsedFieldsIr::Named(ref mut fields) = parsed {
                    validate_gated_fields(fields, &inherited_version)?;
                }

                let fields = VariantFieldsIr::from_parsed(parsed);

                Ok(VariantIr {
                    id,
                    name: ident.to_string(),
                    ident,
                    fields,
                    version: inherited_version,
                })
            })
            .collect::<syn::Result<Vec<_>>>()?;

        // Validate variant IDs form a dense range [0, n).
        let variant_attrs: Vec<VersionAttrs> = variants.iter().map(|v| v.version.clone()).collect();
        validate_dense_ids(&variant_attrs, "variant", input.ident.span())?;

        Ok(Self {
            container,
            variants,
        })
    }

    /// Return variants sorted by their explicit id.
    fn sorted_variants(&self) -> Vec<&VariantIr> {
        let mut sorted: Vec<&VariantIr> = self.variants.iter().collect();
        sorted.sort_by_key(|v| v.id);
        sorted
    }

    fn expand_schema(&self) -> TokenStream {
        let name = self.container.ident();
        let version_tokens = self.container.version().to_version_range_tokens();
        let sorted = self.sorted_variants();
        let variants = sorted
            .iter()
            .map(|variant| variant.fields.expand_schema(variant));

        // Compute variant activation table from inherited variant versions
        // in id order.
        let variant_versions: Vec<_> = sorted.iter().map(|v| v.version.clone()).collect();
        let activation_table = compute_activation_table_tokens(&variant_versions);

        quote! {
            &::kaloron::Schema::Enum(
                ::kaloron::EnumSchema::new(
                    stringify!(#name),
                    &[
                        #( #variants, )*
                    ],
                    #version_tokens,
                    #activation_table,
                )
            )
        }
    }

    fn expand_send(&self) -> TokenStream {
        let name = self.container.ident();
        let helper_generics = self.container.helper_generics();
        let (helper_impl_generics, helper_ty_generics, helper_where_clause) =
            helper_generics.split_for_impl();
        let (_, ty_generics, _) = self.container.generics().split_for_impl();

        let variant_arms = self
            .variants
            .iter()
            .map(|variant| variant.expand_send_arm(name));

        // Generate variant version validation
        let variant_version_checks = self.expand_variant_send_checks(name);

        quote! {
            // Validate the current variant exists at this version
            #variant_version_checks

            #[allow(non_camel_case_types)]
            struct __KaloronSend #helper_impl_generics #helper_where_clause {
                value: &'a #name #ty_generics,
            }

            impl #helper_impl_generics ::kaloron::EnumSend for __KaloronSend #helper_ty_generics #helper_where_clause {
                fn visit(&self, send: &mut impl ::kaloron::EnumSendAccept) -> ::anyhow::Result<()> {
                    match self.value {
                        #( #variant_arms )*
                    }
                }
            }

            send.accept_enum(&__KaloronSend { value: self })
        }
    }

    fn expand_recv(&self) -> TokenStream {
        let helper_generics = self.container.generics().clone();
        let (helper_impl_generics, helper_ty_generics, helper_where_clause) =
            helper_generics.split_for_impl();
        let name = self.container.ident();
        let variant_arms = self
            .variants
            .iter()
            .map(|variant| variant.expand_recv_arm(name));

        quote! {
            #[allow(non_camel_case_types)]
            struct __KaloronRecv #helper_impl_generics #helper_where_clause {
                value: ::std::option::Option<#name #helper_ty_generics>,
                version: ::kaloron::Version,
            }

            impl #helper_impl_generics ::kaloron::EnumRecv for __KaloronRecv #helper_ty_generics #helper_where_clause {
                fn visit(
                    &mut self,
                    index: isize,
                    recv: &mut impl ::kaloron::EnumRecvAccept,
                ) -> ::anyhow::Result<()> {
                    let __es = <#name #helper_ty_generics as ::kaloron::TypeShape>::SCHEMA
                        .as_enum().unwrap();
                    let __va = __es.active_variants_at(self.version)
                        .ok_or_else(|| ::anyhow::anyhow!(
                            "no activation data for version {}", self.version
                        ))?;
                    match index {
                        #( #variant_arms )*
                        _ => ::std::result::Result::Err(::anyhow::anyhow!("variant index out of bounds")),
                    }
                }
            }

            let mut builder = __KaloronRecv {
                value: ::std::option::Option::None,
                version,
            };

            recv.accept_enum(&mut builder)?;
            builder.value.ok_or_else(|| ::anyhow::anyhow!("missing enum value"))
        }
    }

    /// Generate variant version validation for send using schema activation table.
    fn expand_variant_send_checks(&self, enum_name: &Ident) -> TokenStream {
        let check_arms: Vec<TokenStream> = self
            .variants
            .iter()
            .map(|variant| {
                let variant_ident = &variant.ident;
                let variant_name = &variant.name;
                let idx_lit = Literal::usize_unsuffixed(variant.id as usize);

                // Generate a pattern that matches regardless of fields
                let pattern = match &variant.fields {
                    VariantFieldsIr::Unit(_) => quote! { #enum_name::#variant_ident },
                    VariantFieldsIr::Newtype(_) => quote! { #enum_name::#variant_ident(..) },
                    VariantFieldsIr::Tuple(_) => quote! { #enum_name::#variant_ident(..) },
                    VariantFieldsIr::Named(_) => quote! { #enum_name::#variant_ident { .. } },
                };

                quote! {
                    #pattern => {
                        if __va[#idx_lit].is_inactive() {
                            return ::std::result::Result::Err(::anyhow::anyhow!(
                                concat!("variant '", #variant_name, "' does not exist at version {}"),
                                version
                            ));
                        }
                    }
                }
            })
            .collect();

        quote! {
            let __va = <Self as ::kaloron::TypeShape>::SCHEMA
                .as_enum().unwrap()
                .active_variants_at(version)
                .ok_or_else(|| ::anyhow::anyhow!(
                    "no activation data for version {}", version
                ))?;
            match self {
                #(#check_arms)*
            }
        }
    }

    pub(crate) fn expand(&self) -> TokenStream {
        let marker_impl = self.container.expand_marker_impl(quote! {
            ::kaloron::MarkerCompositeTypeShapeEnum
        });
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

impl VariantIr {
    fn expand_send_arm(&self, enum_name: &Ident) -> TokenStream {
        let variant_ident = &self.ident;
        let idx = Literal::isize_unsuffixed(self.id as isize);
        let (binding, accept_call) = match &self.fields {
            VariantFieldsIr::Unit(ir) => ir.expand_send_parts(&idx),
            VariantFieldsIr::Newtype(ir) => ir.expand_send_parts(&idx),
            VariantFieldsIr::Tuple(ir) => ir.expand_send_parts(&idx),
            VariantFieldsIr::Named(ir) => ir.expand_send_parts(&idx),
        };

        quote! {
            #enum_name::#variant_ident #binding => {
                #accept_call
            }
        }
    }

    fn expand_recv_arm(&self, enum_name: &Ident) -> TokenStream {
        let idx = Literal::isize_unsuffixed(self.id as isize);
        let idx_usize = Literal::usize_unsuffixed(self.id as usize);
        let variant_ident = self.ident.clone();
        let variant_name = &self.name;

        let (builder, inner) = match &self.fields {
            VariantFieldsIr::Unit(ir) => ir.expand_recv_value(),
            VariantFieldsIr::Newtype(ir) => ir.expand_recv_value(),
            VariantFieldsIr::Tuple(ir) => ir.expand_recv_value(),
            VariantFieldsIr::Named(ir) => ir.expand_recv_value(self),
        };

        quote! {
            #idx => {
                // Validate variant exists at version using schema activation table
                if __va[#idx_usize].is_inactive() {
                    return ::std::result::Result::Err(::anyhow::anyhow!(
                        concat!("variant '", #variant_name, "' does not exist at version {}"),
                        self.version
                    ));
                }
                #builder
                self.value = ::std::option::Option::Some(#enum_name::#variant_ident #inner );
                ::std::result::Result::Ok(())
            }
        }
    }
}

impl VariantFieldsIr {
    fn from_parsed(parsed: ParsedFieldsIr) -> Self {
        match parsed {
            ParsedFieldsIr::Unit => Self::Unit(UnitVariantIr),
            ParsedFieldsIr::Newtype(ty) => Self::Newtype(NewtypeVariantIr { ty }),
            ParsedFieldsIr::Tuple(fields) => Self::Tuple(TupleVariantIr { fields }),
            ParsedFieldsIr::Named(fields) => Self::Named(NamedVariantIr { fields }),
        }
    }

    fn expand_schema(&self, variant: &VariantIr) -> TokenStream {
        match self {
            Self::Unit(ir) => ir.expand_schema(variant),
            Self::Newtype(ir) => ir.expand_schema(variant),
            Self::Tuple(ir) => ir.expand_schema(variant),
            Self::Named(ir) => ir.expand_schema(variant),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::EnumIr;
    use syn::DeriveInput;

    fn assist_parse_input(source: &str) -> DeriveInput {
        syn::parse_str(source).expect("valid derive input")
    }

    #[test]
    fn test_dispatches_large_tuple_variant_codegen() {
        let fields = std::iter::repeat_n("u8", 17).collect::<Vec<_>>().join(", ");
        let input = assist_parse_input(&format!(
            "#[kaloron(introduced = \"0.0.0\")] enum Big {{ #[kaloron(id = 0)] V({fields}) }}"
        ));
        let data = match &input.data {
            syn::Data::Enum(data) => data,
            _ => panic!("expected enum"),
        };

        let expanded = EnumIr::from(&input, data).unwrap().expand().to_string();
        assert!(expanded.contains("__TbCp"));
        assert!(expanded.contains("__Tb0"));
        assert!(expanded.contains("__TvCp"));
        assert!(expanded.contains("__Tv16"));
        assert!(expanded.contains("accept_enum"));
        assert!(expanded.contains("accept_tuple"));
        assert!(expanded.contains("__tbc"));
    }

    #[test]
    fn test_dispatches_enum_recv_codegen() {
        let input = assist_parse_input(
            "#[kaloron(introduced = \"0.0.0\")] enum Example<T> { \
             #[kaloron(id = 0)] Unit, \
             #[kaloron(id = 1)] Newtype(T), \
             #[kaloron(id = 2)] Tuple(u8, String), \
             #[kaloron(id = 3)] Struct { #[kaloron(id = 0)] id: u32, #[kaloron(id = 1)] inner: T } \
             }",
        );
        let data = match &input.data {
            syn::Data::Enum(data) => data,
            _ => panic!("expected enum"),
        };

        let expanded = EnumIr::from(&input, data).unwrap().expand().to_string();
        assert!(expanded.contains("MarkerCompositeTypeShapeEnum"));
        assert!(expanded.contains("accept_enum"));
        assert!(expanded.contains("accept_unit"));
        assert!(expanded.contains("accept_newtype"));
        assert!(expanded.contains("accept_tuple"));
        assert!(expanded.contains("accept_named"));
        assert!(expanded.contains("__Tv01"));
        assert!(expanded.contains("__Tv02"));
        assert!(expanded.contains("missing enum value"));
    }

    #[test]
    fn test_rejects_missing_variant_id() {
        let input = assist_parse_input("#[kaloron(introduced = \"0.0.0\")] enum Bad { Unit }");
        let data = match &input.data {
            syn::Data::Enum(data) => data,
            _ => panic!("expected enum"),
        };
        assert!(EnumIr::from(&input, data).is_err());
    }

    #[test]
    fn test_rejects_duplicate_variant_id() {
        let input = assist_parse_input(
            "#[kaloron(introduced = \"0.0.0\")] enum Bad { \
             #[kaloron(id = 0)] A, \
             #[kaloron(id = 0)] B \
             }",
        );
        let data = match &input.data {
            syn::Data::Enum(data) => data,
            _ => panic!("expected enum"),
        };
        assert!(EnumIr::from(&input, data).is_err());
    }

    #[test]
    fn test_rejects_non_dense_variant_id() {
        let input = assist_parse_input(
            "#[kaloron(introduced = \"0.0.0\")] enum Bad { \
             #[kaloron(id = 0)] A, \
             #[kaloron(id = 2)] B \
             }",
        );
        let data = match &input.data {
            syn::Data::Enum(data) => data,
            _ => panic!("expected enum"),
        };
        assert!(EnumIr::from(&input, data).is_err());
    }
}
