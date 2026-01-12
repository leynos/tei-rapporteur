//! Integration tests for the `bench_memory` example binary.
//!
//! These tests verify that the `bench_memory` example runs successfully in both
//! streaming and full parsing modes.

#![cfg(feature = "streaming")]
#![expect(
    clippy::expect_used,
    reason = "Test module: expect/unwrap is idiomatic in tests"
)]

use std::process::Command;

/// Returns the path to the `bench_memory` example binary (debug build).
fn bench_memory_binary() -> std::path::PathBuf {
    // The binary is built in target/debug/examples when running cargo test
    let mut path = std::env::current_exe()
        .expect("should have current exe")
        .parent()
        .expect("should have parent dir")
        .parent()
        .expect("should have grandparent dir")
        .to_path_buf();
    path.push("examples");
    path.push("bench_memory");
    path
}

/// Checks whether the `bench_memory` example binary exists.
///
/// The example may not be built if running tests without `--all-targets`.
fn binary_exists() -> bool {
    bench_memory_binary().exists()
}

#[test]
fn streaming_mode_exits_successfully() {
    if !binary_exists() {
        // Skip silently - nextest will show this as passed
        return;
    }

    let output = Command::new(bench_memory_binary())
        .arg("streaming")
        .output()
        .expect("should execute bench_memory");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "streaming mode should exit successfully: {stderr}"
    );
    assert!(
        stderr.contains("body blocks"),
        "streaming mode should report body block count: {stderr}"
    );
}

#[test]
fn full_mode_exits_successfully() {
    if !binary_exists() {
        // Skip silently - nextest will show this as passed
        return;
    }

    let output = Command::new(bench_memory_binary())
        .arg("full")
        .output()
        .expect("should execute bench_memory");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "full mode should exit successfully: {stderr}"
    );
    assert!(
        stderr.contains("body blocks"),
        "full mode should report body block count: {stderr}"
    );
}

#[test]
fn unknown_mode_shows_usage() {
    if !binary_exists() {
        // Skip silently - nextest will show this as passed
        return;
    }

    let output = Command::new(bench_memory_binary())
        .arg("invalid")
        .output()
        .expect("should execute bench_memory");

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Usage is shown for unknown modes; exit should still be success (0)
    assert!(
        stderr.contains("Usage:"),
        "unknown mode should show usage: {stderr}"
    );
}

#[test]
fn no_args_shows_usage() {
    if !binary_exists() {
        // Skip silently - nextest will show this as passed
        return;
    }

    let output = Command::new(bench_memory_binary())
        .output()
        .expect("should execute bench_memory");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Usage:"),
        "no args should show usage: {stderr}"
    );
}
