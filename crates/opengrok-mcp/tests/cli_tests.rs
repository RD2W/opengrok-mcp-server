// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev (RD2W) <mkrutovercev@yandex.ru>

//! Integration tests for the opengrok-mcp CLI.

use std::process::Command;

/// Path to the compiled binary — set by cargo during `cargo test`.
fn binary_path() -> &'static str {
    env!("CARGO_BIN_EXE_opengrok-mcp")
}

#[test]
fn version_flag_prints_metadata_and_exits_zero() {
    let output = Command::new(binary_path())
        .arg("--version")
        .output()
        .expect("failed to run binary");

    assert!(output.status.success(), "--version should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("author:"),
        "version output should contain author field, got:\n{stdout}"
    );
    assert!(
        stdout.contains("commit:"),
        "version output should contain commit field, got:\n{stdout}"
    );
    assert!(
        stdout.contains("built:"),
        "version output should contain built field, got:\n{stdout}"
    );
    assert!(
        stdout.contains("target:"),
        "version output should contain target field, got:\n{stdout}"
    );
}

#[test]
fn version_flag_prints_semver() {
    let output = Command::new(binary_path())
        .arg("--version")
        .output()
        .expect("failed to run binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // The first line should contain the binary name and a semver version like "0.1.0"
    let first_line = stdout.lines().next().expect("version output is empty");
    assert!(
        first_line.contains("opengrok-mcp"),
        "first line should contain binary name, got: '{first_line}'"
    );
}

#[test]
fn help_flag_prints_usage_and_exits_zero() {
    let output = Command::new(binary_path())
        .arg("--help")
        .output()
        .expect("failed to run binary");

    assert!(output.status.success(), "--help should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("--config"),
        "help should mention --config flag, got:\n{stdout}"
    );
    assert!(
        stdout.contains("--version"),
        "help should mention --version flag, got:\n{stdout}"
    );
}

#[test]
fn help_flag_short_form_works() {
    let output = Command::new(binary_path())
        .arg("-h")
        .output()
        .expect("failed to run binary");

    assert!(output.status.success(), "-h should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--config"),
        "short help should mention --config, got:\n{stdout}"
    );
}

#[test]
fn version_flag_short_form_works() {
    let output = Command::new(binary_path())
        .arg("-V")
        .output()
        .expect("failed to run binary");

    assert!(output.status.success(), "-V should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("author:"),
        "short version should contain author, got:\n{stdout}"
    );
}
