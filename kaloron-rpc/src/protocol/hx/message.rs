// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use kaloron::TypeShape;

#[derive(TypeShape)]
#[kaloron(introduced = "0.0.0")]
pub(super) enum HxServerBoundHeader {
    #[kaloron(id = 0)]
    Request {
        #[kaloron(id = 0)]
        call_id: u32,
        #[kaloron(id = 1)]
        method_id: u32,
    },
    #[kaloron(id = 1)]
    Complete {},
}

#[derive(TypeShape)]
#[kaloron(introduced = "0.0.0")]
pub(super) enum HxClientBoundHeader {
    #[kaloron(id = 0)]
    ResponseSuccess {
        #[kaloron(id = 0)]
        call_id: u32,
    },
    #[kaloron(id = 1)]
    ResponseFailure {
        #[kaloron(id = 0)]
        call_id: u32,
    },
    #[kaloron(id = 2)]
    ChannelError {},
    #[kaloron(id = 3)]
    Complete {},
}
