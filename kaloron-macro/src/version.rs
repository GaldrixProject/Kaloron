// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::quote;
use std::collections::BTreeSet;
use syn::spanned::Spanned;
use syn::{Attribute, Error, Result};

/// Parsed version number for use in macro expansion.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Version {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
    pub build: u16,
}

impl Version {
    #[allow(dead_code)]
    pub const ZERO: Self = Self {
        major: 0,
        minor: 0,
        patch: 0,
        build: 0,
    };

    /// Parse from string "MAJOR.MINOR.PATCH" or "MAJOR.MINOR.PATCH.BUILD".
    pub fn parse(s: &str, span: proc_macro2::Span) -> Result<Self> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() < 3 || parts.len() > 4 {
            return Err(Error::new(
                span,
                format!(
                    "invalid version format '{}': expected MAJOR.MINOR.PATCH or MAJOR.MINOR.PATCH.BUILD",
                    s
                ),
            ));
        }

        let parse_u16 = |s: &str| -> Result<u16> {
            s.parse::<u16>()
                .map_err(|e| Error::new(span, format!("invalid version component '{}': {}", s, e)))
        };

        Ok(Self {
            major: parse_u16(parts[0])?,
            minor: parse_u16(parts[1])?,
            patch: parse_u16(parts[2])?,
            build: if parts.len() == 4 {
                parse_u16(parts[3])?
            } else {
                0
            },
        })
    }

    /// Generate code to create a runtime Version from this parsed version.
    pub fn to_tokens(self) -> TokenStream {
        let major = self.major;
        let minor = self.minor;
        let patch = self.patch;
        let build = self.build;
        quote! {
            ::kaloron::Version::new(#major, #minor, #patch, #build)
        }
    }

    /// Return the next representable version in packed order.
    pub fn successor(&self) -> Option<Self> {
        if self.build < u16::MAX {
            Some(Self {
                major: self.major,
                minor: self.minor,
                patch: self.patch,
                build: self.build + 1,
            })
        } else if self.patch < u16::MAX {
            Some(Self {
                major: self.major,
                minor: self.minor,
                patch: self.patch + 1,
                build: 0,
            })
        } else if self.minor < u16::MAX {
            Some(Self {
                major: self.major,
                minor: self.minor + 1,
                patch: 0,
                build: 0,
            })
        } else if self.major < u16::MAX {
            Some(Self {
                major: self.major + 1,
                minor: 0,
                patch: 0,
                build: 0,
            })
        } else {
            None
        }
    }

    pub fn format(&self) -> String {
        if self.build == 0 {
            format!("{}.{}.{}", self.major, self.minor, self.patch)
        } else {
            format!(
                "{}.{}.{}.{}",
                self.major, self.minor, self.patch, self.build
            )
        }
    }
}

/// A single deprecation span parsed from attributes.
#[derive(Clone, Debug)]
pub(crate) struct DeprecationSpan {
    pub since: Version,
    pub until: Option<Version>,
}

impl DeprecationSpan {
    /// Parse from string "X.Y.Z" or "X.Y.Z..A.B.C".
    pub fn parse(s: &str, span: proc_macro2::Span) -> Result<Self> {
        if let Some(pos) = s.find("..") {
            let left = &s[..pos];
            let right = &s[pos + 2..];
            if left.is_empty() || right.is_empty() {
                return Err(Error::new(
                    span,
                    "invalid deprecation range format: expected VERSION..VERSION",
                ));
            }
            Ok(Self {
                since: Version::parse(left, span)?,
                until: Some(Version::parse(right, span)?),
            })
        } else {
            Ok(Self {
                since: Version::parse(s, span)?,
                until: None,
            })
        }
    }

    /// Generate code to create a runtime DeprecationSpan.
    pub fn to_tokens(&self) -> TokenStream {
        let since = self.since.to_tokens();
        if let Some(until) = &self.until {
            let until = until.to_tokens();
            quote! {
                ::kaloron::DeprecationSpan::new_range(#since, #until)
            }
        } else {
            quote! {
                ::kaloron::DeprecationSpan::new(#since)
            }
        }
    }
}

/// Parsed version attributes from `#[kaloron(...)]` attributes.
#[derive(Clone, Debug, Default)]
pub(crate) struct VersionAttrs {
    pub introduced: Option<Version>,
    pub deprecations: Vec<DeprecationSpan>,
    pub removed_in: Option<Version>,
    /// Explicit field/variant ID for wire stability.
    pub id: Option<u32>,
}

impl VersionAttrs {
    /// Merge with parent's version attrs (inheritance).
    /// Child values take precedence over inherited values.
    /// Note: `id` is never inherited from parent.
    pub fn inherit_from(&self, parent: &VersionAttrs) -> VersionAttrs {
        VersionAttrs {
            introduced: self.introduced.or(parent.introduced),
            deprecations: if self.deprecations.is_empty() {
                parent.deprecations.clone()
            } else {
                self.deprecations.clone()
            },
            removed_in: self.removed_in.or(parent.removed_in),
            id: self.id,
        }
    }

    /// Generate code to create a runtime VersionRange.
    pub fn to_version_range_tokens(&self) -> TokenStream {
        let intro = match &self.introduced {
            Some(v) => v.to_tokens(),
            None => quote! { ::kaloron::Version::zero() },
        };
        let deprecations = if self.deprecations.is_empty() {
            quote! { &[] }
        } else {
            let spans: Vec<TokenStream> = self.deprecations.iter().map(|s| s.to_tokens()).collect();
            quote! { &[#(#spans),*] }
        };
        let removed = match &self.removed_in {
            Some(v) => {
                let v_tokens = v.to_tokens();
                quote! { ::std::option::Option::Some(#v_tokens) }
            }
            None => quote! { ::std::option::Option::None },
        };

        quote! {
            ::kaloron::VersionRange {
                introduced: #intro,
                deprecations: #deprecations,
                removed_in: #removed,
            }
        }
    }

    /// Check if this item exists at the given macro-side version.
    /// Uses the same semantics as `VersionRange::exists_at`.
    pub fn exists_at(&self, version: &Version) -> bool {
        let intro = self.introduced.unwrap_or(Version::ZERO);
        *version >= intro
            && match self.removed_in {
                Some(r) => *version < r,
                None => true,
            }
    }

    /// Check if this item is deprecated at the given macro-side version.
    pub fn is_deprecated_at(&self, version: &Version) -> bool {
        self.deprecations.iter().any(|dep| {
            let after_start = *version >= dep.since;
            let before_end = match dep.until {
                Some(until) => *version <= until,
                None => true,
            };
            after_start && before_end
        })
    }
}

/// Compute the version activation table tokens for a list of items
/// (fields or variants) with inherited version attrs.
///
/// Returns a token stream representing `&[VersionActivationEntry { ... }, ...]`
/// with entries ordered by version ascending. Each entry has a three-state
/// array where the index corresponds to the item index.
pub(crate) fn compute_activation_table_tokens(item_versions: &[VersionAttrs]) -> TokenStream {
    // Collect all "change point" versions where activation state may change.
    let mut change_points: BTreeSet<Version> = BTreeSet::new();

    for iv in item_versions {
        if let Some(intro) = iv.introduced {
            change_points.insert(intro);
        }
        if let Some(removed) = iv.removed_in {
            change_points.insert(removed);
        }
        for dep in &iv.deprecations {
            change_points.insert(dep.since);
            if let Some(until) = dep.until {
                if let Some(after_until) = until.successor() {
                    change_points.insert(after_until);
                }
            }
        }
    }

    if change_points.is_empty() {
        return quote! { &[] };
    }

    // For each change point, compute the activation state for each item.
    let entries: Vec<TokenStream> = change_points
        .iter()
        .map(|version| {
            let version_tokens = version.to_tokens();
            let states: Vec<TokenStream> = item_versions
                .iter()
                .map(|iv| {
                    if !iv.exists_at(version) {
                        quote! { ::kaloron::ActivationState::Inactive }
                    } else if iv.is_deprecated_at(version) {
                        quote! { ::kaloron::ActivationState::Deprecated }
                    } else {
                        quote! { ::kaloron::ActivationState::Active }
                    }
                })
                .collect();
            quote! {
                ::kaloron::VersionActivationEntry {
                    version: #version_tokens,
                    active: &[
                        #(#states,)*
                    ],
                }
            }
        })
        .collect();

    quote! {
        &[
            #(#entries,)*
        ]
    }
}

/// Parse all `#[kaloron(...)]` attributes from a list of attributes.
pub(crate) fn parse_kaloron_attrs(attrs: &[Attribute]) -> Result<VersionAttrs> {
    let mut result = VersionAttrs::default();

    for attr in attrs {
        if !attr.path().is_ident("kaloron") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            let span = meta.path.span();

            if meta.path.is_ident("introduced") {
                let value = meta.value()?.parse::<syn::LitStr>()?;
                if result.introduced.is_some() {
                    return Err(Error::new(span, "duplicate 'introduced' attribute"));
                }
                result.introduced = Some(Version::parse(&value.value(), value.span())?);
                Ok(())
            } else if meta.path.is_ident("deprecated") {
                let value = meta.value()?.parse::<syn::LitStr>()?;
                result
                    .deprecations
                    .push(DeprecationSpan::parse(&value.value(), value.span())?);
                Ok(())
            } else if meta.path.is_ident("removed") {
                let value = meta.value()?.parse::<syn::LitStr>()?;
                if result.removed_in.is_some() {
                    return Err(Error::new(span, "duplicate 'removed' attribute"));
                }
                result.removed_in = Some(Version::parse(&value.value(), value.span())?);
                Ok(())
            } else if meta.path.is_ident("id") {
                let value = meta.value()?.parse::<syn::LitInt>()?;
                if result.id.is_some() {
                    return Err(Error::new(span, "duplicate 'id' attribute"));
                }
                result.id = Some(value.base10_parse::<u32>()?);
                Ok(())
            } else {
                Err(meta.error(
                    "unknown kaloron attribute; expected `introduced`, `deprecated`, `removed`, or `id`",
                ))
            }
        })?;
    }

    Ok(result)
}

/// Validate version specification for consistency.
pub(crate) fn validate_version_spec(spec: &VersionAttrs, span: proc_macro2::Span) -> Result<()> {
    // Check introduced <= removed_in
    if let (Some(intro), Some(removed)) = (spec.introduced, spec.removed_in) {
        if intro > removed {
            return Err(Error::new(
                span,
                format!(
                    "removed_in ({}) must be >= introduced ({})",
                    removed.format(),
                    intro.format()
                ),
            ));
        }
    }

    // Check introduced <= deprecation.since
    for dep in &spec.deprecations {
        if let Some(intro) = spec.introduced {
            if dep.since < intro {
                return Err(Error::new(
                    span,
                    format!(
                        "deprecation since ({}) must be >= introduced ({})",
                        dep.since.format(),
                        intro.format()
                    ),
                ));
            }
        }

        // Check since <= until
        if let Some(until) = dep.until {
            if dep.since > until {
                return Err(Error::new(
                    span,
                    format!(
                        "deprecation until ({}) must be >= since ({})",
                        until.format(),
                        dep.since.format()
                    ),
                ));
            }
        }
    }

    // Check for overlapping deprecation spans
    for i in 0..spec.deprecations.len() {
        for j in (i + 1)..spec.deprecations.len() {
            if spans_overlap(&spec.deprecations[i], &spec.deprecations[j]) {
                return Err(Error::new(
                    span,
                    format!(
                        "deprecation spans overlap: [{}] and [{}]",
                        format_span(&spec.deprecations[i]),
                        format_span(&spec.deprecations[j])
                    ),
                ));
            }
        }
    }

    Ok(())
}

fn spans_overlap(a: &DeprecationSpan, b: &DeprecationSpan) -> bool {
    let a_end = a.until.unwrap_or(Version {
        major: u16::MAX,
        minor: u16::MAX,
        patch: u16::MAX,
        build: u16::MAX,
    });
    let b_end = b.until.unwrap_or(Version {
        major: u16::MAX,
        minor: u16::MAX,
        patch: u16::MAX,
        build: u16::MAX,
    });
    a.since <= b_end && b.since <= a_end
}

fn format_span(s: &DeprecationSpan) -> String {
    if let Some(until) = s.until {
        format!("{}..{}", s.since.format(), until.format())
    } else {
        s.since.format()
    }
}

/// Validate that a set of IDs forms a dense range [0, n).
/// Each item must have an explicit `id` attribute, all IDs must be unique,
/// and together they must cover exactly 0, 1, ..., n-1.
pub(crate) fn validate_dense_ids(
    attrs: &[VersionAttrs],
    item_kind: &str,
    span: proc_macro2::Span,
) -> Result<()> {
    let n = attrs.len() as u32;
    let mut seen = vec![false; n as usize];

    for (i, a) in attrs.iter().enumerate() {
        let id = match a.id {
            Some(id) => id,
            None => {
                return Err(Error::new(
                    span,
                    format!(
                        "{item_kind} at position {i} is missing a #[kaloron(id = N)] attribute"
                    ),
                ))
            }
        };
        if id >= n {
            return Err(Error::new(
                span,
                format!(
                    "{item_kind} id {id} is out of range; with {n} {item_kind}s, ids must be in [0, {n})"
                ),
            ));
        }
        if seen[id as usize] {
            return Err(Error::new(span, format!("duplicate {item_kind} id {id}")));
        }
        seen[id as usize] = true;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parse_valid() {
        let span = proc_macro2::Span::call_site();
        assert_eq!(
            Version::parse("1.2.3", span).unwrap(),
            Version {
                major: 1,
                minor: 2,
                patch: 3,
                build: 0
            }
        );
        assert_eq!(
            Version::parse("1.2.3.4", span).unwrap(),
            Version {
                major: 1,
                minor: 2,
                patch: 3,
                build: 4
            }
        );
    }

    #[test]
    fn test_version_parse_invalid() {
        let span = proc_macro2::Span::call_site();
        assert!(Version::parse("1.2", span).is_err());
        assert!(Version::parse("1.2.3.4.5", span).is_err());
        assert!(Version::parse("abc.0.0", span).is_err());
    }

    #[test]
    fn test_deprecation_span_parse_open() {
        let span = proc_macro2::Span::call_site();
        let dep = DeprecationSpan::parse("2.0.0", span).unwrap();
        assert_eq!(
            dep.since,
            Version {
                major: 2,
                minor: 0,
                patch: 0,
                build: 0
            }
        );
        assert!(dep.until.is_none());
    }

    #[test]
    fn test_deprecation_span_parse_range() {
        let span = proc_macro2::Span::call_site();
        let dep = DeprecationSpan::parse("2.0.0..2.1.0", span).unwrap();
        assert_eq!(
            dep.since,
            Version {
                major: 2,
                minor: 0,
                patch: 0,
                build: 0
            }
        );
        assert_eq!(
            dep.until,
            Some(Version {
                major: 2,
                minor: 1,
                patch: 0,
                build: 0
            })
        );
    }

    #[test]
    fn test_validate_ordering() {
        let span = proc_macro2::Span::call_site();

        // introduced > removed → should fail
        let spec = VersionAttrs {
            introduced: Some(Version {
                major: 2,
                minor: 0,
                patch: 0,
                build: 0,
            }),
            deprecations: vec![],
            removed_in: Some(Version {
                major: 1,
                minor: 0,
                patch: 0,
                build: 0,
            }),
            id: None,
        };
        assert!(validate_version_spec(&spec, span).is_err());

        // Valid ordering
        let spec = VersionAttrs {
            introduced: Some(Version {
                major: 1,
                minor: 0,
                patch: 0,
                build: 0,
            }),
            deprecations: vec![],
            removed_in: Some(Version {
                major: 2,
                minor: 0,
                patch: 0,
                build: 0,
            }),
            id: None,
        };
        assert!(validate_version_spec(&spec, span).is_ok());
    }

    #[test]
    fn test_validate_overlap() {
        let span = proc_macro2::Span::call_site();
        let spec = VersionAttrs {
            introduced: Some(Version {
                major: 1,
                minor: 0,
                patch: 0,
                build: 0,
            }),
            deprecations: vec![
                DeprecationSpan {
                    since: Version {
                        major: 2,
                        minor: 0,
                        patch: 0,
                        build: 0,
                    },
                    until: Some(Version {
                        major: 2,
                        minor: 2,
                        patch: 0,
                        build: 0,
                    }),
                },
                DeprecationSpan {
                    since: Version {
                        major: 2,
                        minor: 1,
                        patch: 0,
                        build: 0,
                    },
                    until: Some(Version {
                        major: 2,
                        minor: 3,
                        patch: 0,
                        build: 0,
                    }),
                },
            ],
            removed_in: None,
            id: None,
        };
        let err = validate_version_spec(&spec, span).unwrap_err();
        assert!(err.to_string().contains("overlap"));
    }

    #[test]
    fn test_inheritance() {
        let parent = VersionAttrs {
            introduced: Some(Version {
                major: 1,
                minor: 0,
                patch: 0,
                build: 0,
            }),
            deprecations: vec![DeprecationSpan {
                since: Version {
                    major: 2,
                    minor: 0,
                    patch: 0,
                    build: 0,
                },
                until: None,
            }],
            removed_in: Some(Version {
                major: 3,
                minor: 0,
                patch: 0,
                build: 0,
            }),
            id: None,
        };

        // Child with no attrs inherits everything
        let child = VersionAttrs::default();
        let inherited = child.inherit_from(&parent);
        assert_eq!(inherited.introduced, parent.introduced);
        assert_eq!(inherited.deprecations.len(), 1);
        assert_eq!(inherited.removed_in, parent.removed_in);

        // Child with own introduced overrides
        let child = VersionAttrs {
            introduced: Some(Version {
                major: 1,
                minor: 2,
                patch: 0,
                build: 0,
            }),
            ..Default::default()
        };
        let inherited = child.inherit_from(&parent);
        assert_eq!(
            inherited.introduced,
            Some(Version {
                major: 1,
                minor: 2,
                patch: 0,
                build: 0
            })
        );
        // Inherits deprecation from parent
        assert_eq!(inherited.deprecations.len(), 1);
        // Inherits removed from parent
        assert_eq!(inherited.removed_in, parent.removed_in);
    }
}
