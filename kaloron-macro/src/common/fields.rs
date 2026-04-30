// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use crate::version::{parse_kaloron_attrs, validate_dense_ids, VersionAttrs};
use proc_macro2::{Ident, Literal, TokenStream};
use quote::quote;
use syn::{Fields, Type};

#[derive(Clone)]
pub(crate) struct NamedFieldIr {
    pub(crate) id: u32,
    pub(crate) ident: Ident,
    pub(crate) ty: Type,
    pub(crate) name: String,
    pub(crate) version: VersionAttrs,
    /// Whether this field uses Gated<T> wrapping (can be void at some version).
    pub(crate) is_gated: bool,
}

impl NamedFieldIr {
    /// Return fields sorted by their explicit id.
    pub(crate) fn sorted_by_id(v: &[NamedFieldIr]) -> Vec<&NamedFieldIr> {
        let mut sorted: Vec<&NamedFieldIr> = v.iter().collect();
        sorted.sort_by_key(|f| f.id);
        sorted
    }

    /// Collect actual field types in id order (including Gated<T> for gated fields).
    /// Used for builder layouts (send/recv).
    pub(crate) fn collect_types(v: &[NamedFieldIr]) -> Vec<&Type> {
        Self::sorted_by_id(v).iter().map(|f| &f.ty).collect()
    }

    /// Collect field idents in id order.
    pub(crate) fn collect_names(v: &[NamedFieldIr]) -> Vec<&Ident> {
        Self::sorted_by_id(v).iter().map(|f| &f.ident).collect()
    }

    /// Generate TokenStream with the Field definitions for a named struct/variant.
    /// Fields are emitted in id order. Applies version inheritance from `parent_version`.
    /// For gated fields, uses the inner type (unwrapped from Gated<T>) for the schema.
    pub(crate) fn collect_fields(v: &[NamedFieldIr], parent_version: &VersionAttrs) -> TokenStream {
        let sorted = Self::sorted_by_id(v);
        let defs: Vec<TokenStream> = sorted
            .iter()
            .map(|field| {
                let id_lit = Literal::u32_unsuffixed(field.id);
                let schema_ty = if field.is_gated {
                    unwrap_gated_inner(&field.ty)
                } else {
                    &field.ty
                };
                let field_name = Literal::string(&field.name);
                let inherited = field.version.inherit_from(parent_version);
                let version_tokens = inherited.to_version_range_tokens();
                quote! {
                    ::kaloron::Field::new_typed_with_version::<#schema_ty>(
                        #id_lit,
                        #field_name,
                        #version_tokens,
                    )
                }
            })
            .collect();
        quote! { #( #defs, )* }
    }

    /// Generate TokenStream with the (name, index) pairs sorted by name.
    /// The index value is the field's explicit id.
    pub(crate) fn collect_index(v: &[NamedFieldIr]) -> TokenStream {
        let mut index_pairs: Vec<(&str, u32)> = v.iter().map(|f| (f.name.as_str(), f.id)).collect();
        index_pairs.sort_by(|a, b| a.0.cmp(b.0));

        let defs: Vec<TokenStream> = index_pairs
            .iter()
            .map(|(name, id)| {
                let name_lit = Literal::string(name);
                let id_lit = Literal::usize_unsuffixed(*id as usize);
                quote! { (#name_lit, #id_lit) }
            })
            .collect();

        quote! { #( #defs, )* }
    }
}

#[derive(Clone)]
pub(crate) enum ParsedFieldsIr {
    Unit,
    Newtype(Type),
    Tuple(Vec<Type>),
    Named(Vec<NamedFieldIr>),
}

#[derive(Copy, Clone)]
pub(crate) enum FieldsOwner {
    TupleStruct,
    TupleVariant,
}

impl FieldsOwner {
    fn tuple_limit_message(self) -> &'static str {
        match self {
            Self::TupleStruct => "derive(TypeShape) supports tuple structs with at most 32 fields",
            Self::TupleVariant => {
                "derive(TypeShape) supports tuple variants with at most 32 fields"
            }
        }
    }
}

pub(crate) fn parse_fields(fields: &Fields, owner: FieldsOwner) -> syn::Result<ParsedFieldsIr> {
    match fields {
        Fields::Unit => Ok(ParsedFieldsIr::Unit),
        Fields::Unnamed(unnamed) if unnamed.unnamed.len() == 1 => {
            Ok(ParsedFieldsIr::Newtype(unnamed.unnamed[0].ty.clone()))
        }
        Fields::Unnamed(unnamed) => {
            if unnamed.unnamed.len() > 32 {
                return Err(syn::Error::new_spanned(
                    unnamed,
                    owner.tuple_limit_message(),
                ));
            }

            Ok(ParsedFieldsIr::Tuple(
                unnamed
                    .unnamed
                    .iter()
                    .map(|field| field.ty.clone())
                    .collect(),
            ))
        }
        Fields::Named(named) => {
            let fields_ir: Vec<NamedFieldIr> = named
                .named
                .iter()
                .map(|field| {
                    let ident = field.ident.clone().expect("named field");
                    let attrs = parse_kaloron_attrs(&field.attrs)?;
                    let id = attrs.id.ok_or_else(|| {
                        syn::Error::new_spanned(
                            field,
                            "named fields require a #[kaloron(id = N)] attribute",
                        )
                    })?;
                    Ok(NamedFieldIr {
                        id,
                        name: ident.to_string(),
                        ident,
                        ty: field.ty.clone(),
                        version: attrs,
                        is_gated: false, // will be set in validate_gated_fields
                    })
                })
                .collect::<syn::Result<Vec<_>>>()?;

            // Validate field IDs form a dense range [0, n).
            let field_attrs: Vec<VersionAttrs> =
                fields_ir.iter().map(|f| f.version.clone()).collect();
            validate_dense_ids(&field_attrs, "field", named.brace_token.span.join())?;

            Ok(ParsedFieldsIr::Named(fields_ir))
        }
    }
}

// ---------------------------------------------------------------------------
// Gated<T> detection and validation
// ---------------------------------------------------------------------------

/// Check if a type is syntactically `Gated<T>` (or `path::Gated<T>`).
pub(crate) fn is_gated_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Gated"
                && matches!(segment.arguments, syn::PathArguments::AngleBracketed(_));
        }
    }
    false
}

/// Extract the inner type `T` from a `Gated<T>` type.
/// Panics if the type is not `Gated<T>`.
pub(crate) fn unwrap_gated_inner(ty: &Type) -> &Type {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Gated" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        return inner;
                    }
                }
            }
        }
    }
    panic!("expected Gated<T> type but got something else");
}

/// Determine if a field can be void at some version where the container exists.
/// This happens when the field's version range doesn't fully cover the container's.
pub(crate) fn field_can_be_void(
    container_version: &VersionAttrs,
    field_version: &VersionAttrs,
) -> bool {
    let inherited = field_version.inherit_from(container_version);

    let c_intro = container_version.introduced;
    let f_intro = inherited.introduced;

    // Field introduced later than container
    if f_intro > c_intro {
        return true;
    }

    // Field removed earlier than container (or field removed but container not)
    match (inherited.removed_in, container_version.removed_in) {
        (Some(_fr), None) => return true,
        (Some(fr), Some(cr)) if fr < cr => return true,
        _ => {}
    }

    false
}

/// Validate and mark gated fields. Must be called after parse_fields and
/// with the container's version attrs.
pub(crate) fn validate_gated_fields(
    fields: &mut [NamedFieldIr],
    container_version: &VersionAttrs,
) -> syn::Result<()> {
    for field in fields.iter_mut() {
        let can_void = field_can_be_void(container_version, &field.version);
        let is_gated_ty = is_gated_type(&field.ty);

        if can_void && !is_gated_ty {
            return Err(syn::Error::new_spanned(
                &field.ident,
                format!(
                    "field '{}' can be void at some version and must use Gated<T> type",
                    field.name
                ),
            ));
        }
        if !can_void && is_gated_ty {
            return Err(syn::Error::new_spanned(
                &field.ident,
                format!(
                    "field '{}' is always active and must not use Gated<T> type",
                    field.name
                ),
            ));
        }

        field.is_gated = can_void;
    }
    Ok(())
}
