//! Smoke test for the `cell80` binary shim (`src/bin/cell80.rs`).
//!
//! The shim is a thin wrapper over `cell80::run_cli` (which is unit-tested); this
//! drives the actual built binary so the `main()` dispatch — both the success and error
//! arms — is exercised end-to-end.

use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_cell80"))
}

#[test]
fn run_subcommand_compiles_and_reports() {
    let src = std::env::temp_dir().join("rustz80_cli_smoke_add.rs");
    std::fs::write(
        &src,
        "//! add two\nfn run(a: u16, b: u16) -> u16 { a + b }\n",
    )
    .unwrap();

    let out = bin()
        .args(["run", src.to_str().unwrap(), "--args", "2,3"])
        .output()
        .expect("spawn cell80");

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "exit failure; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        stdout.contains("0x0005"),
        "expected result 5 in output, got:\n{stdout}"
    );
}

#[test]
fn bad_invocation_exits_nonzero_with_message() {
    let out = bin().output().expect("spawn cell80"); // no args → usage error
    assert!(
        !out.status.success(),
        "expected non-zero exit on empty args"
    );
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("cell80:"),
        "expected an error prefix on stderr"
    );
}
