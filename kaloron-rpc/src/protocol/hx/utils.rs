// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use hyper::header::{HeaderName, HeaderValue};

#[derive(Clone, Debug)]
pub(super) struct HeaderPair {
    pub name: HeaderName,
    pub value: HeaderValue,
}

impl HeaderPair {
    pub fn new(name: HeaderName, value: HeaderValue) -> Self {
        Self { name, value }
    }
}
