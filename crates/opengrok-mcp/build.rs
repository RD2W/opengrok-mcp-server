// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev (RD2W) <mkrutovercev@yandex.ru>

//! Build script — injects git commit hash and build date for --version output.
//!
//! Priority chain for GIT_HASH:
//! 1. `GIT_HASH` env var (set by CI / Docker build-arg)
//! 2. `git rev-parse --short=8 HEAD` (local dev)
//! 3. `"unknown"` (fallback)
//!
//! Priority chain for BUILD_DATE:
//! 1. `BUILD_DATE` env var (set by CI / Docker build-arg)
//! 2. Current UTC date in ISO 8601 format

use std::process::Command;

fn main() {
    let git_hash = std::env::var("GIT_HASH").unwrap_or_else(|_| {
        Command::new("git")
            .args(["rev-parse", "--short=8", "HEAD"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    });

    let build_date = std::env::var("BUILD_DATE")
        .unwrap_or_else(|_| chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string());

    let build_target = std::env::var("TARGET").unwrap_or_else(|_| "unknown".to_string());

    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
    println!("cargo:rustc-env=BUILD_DATE={}", build_date);
    println!("cargo:rustc-env=BUILD_TARGET={}", build_target);

    // Re-run build script when git HEAD changes (local dev)
    println!("cargo:rerun-if-changed=.git/HEAD");
}
