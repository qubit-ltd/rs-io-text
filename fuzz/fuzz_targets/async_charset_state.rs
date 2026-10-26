// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
#![no_main]

use libfuzzer_sys::fuzz_target;
use qubit_io_text_fuzz::async_charset_state;

fuzz_target!(|data: &[u8]| {
    async_charset_state::run(data);
});
