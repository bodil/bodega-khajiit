// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::env::var;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if var("NODE_ENV") == Ok("production".to_owned()) || var("INSTANCE_TYPE").is_ok() {
        println!("cargo:rustc-cfg=production");
    }
}
