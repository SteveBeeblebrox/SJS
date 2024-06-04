// From deno:cli/util/checksum.rs
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use std::hash::{Hash,Hasher};

pub fn gen(v: &[impl AsRef<[u8]>]) -> String {
    let mut hasher = twox_hash::XxHash64::default();
    hasher.write(v);
    return format!("{:x}",hasher.finish());
}