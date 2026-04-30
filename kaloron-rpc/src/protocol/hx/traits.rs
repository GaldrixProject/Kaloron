// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use std::future::Future;

pub trait HyperStream: hyper::rt::Read + hyper::rt::Write + Unpin + Send + 'static {}

impl<T> HyperStream for T where T: hyper::rt::Read + hyper::rt::Write + Unpin + Send + 'static {}

pub trait HyperListener: Send + Sync + 'static {
    type Io: HyperStream;

    fn accept(&self) -> impl Future<Output = anyhow::Result<Option<Self::Io>>> + Send + '_;

    fn complete(&self) -> impl Future<Output = ()> + Send + '_;
}
