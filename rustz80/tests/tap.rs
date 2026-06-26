//! `.tap` emitter tests (offline — `rustz80` only). The ROM-gated *boot on a real
//! Spectrum* test lives in chuk-speccy (`speccy-sdk/tests/tap_boot.rs`), which has the
//! full emulator + ROM and depends on this crate.

/// Split a `.tap` into its blocks' inner data (flag + payload, checksum stripped).
fn blocks(tap: &[u8]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut i = 0;
    while i + 2 <= tap.len() {
        let len = u16::from_le_bytes([tap[i], tap[i + 1]]) as usize;
        let block = &tap[i + 2..i + 2 + len];
        // Verify the XOR checksum (last byte) over the rest.
        let sum = block[..block.len() - 1].iter().fold(0u8, |a, &b| a ^ b);
        assert_eq!(sum, block[block.len() - 1], "bad checksum");
        out.push(block[..block.len() - 1].to_vec()); // flag + data, no checksum
        i += 2 + len;
    }
    assert_eq!(i, tap.len(), "trailing bytes");
    out
}

#[test]
fn tap_structure() {
    let code = [0x21, 0x2A, 0x00, 0xC9]; // LD HL,42 ; RET
    let tap = rustz80::to_tap(&code, 0x8000, 0x8000, "DEMO");
    let b = blocks(&tap);
    assert_eq!(b.len(), 4, "BASIC header+data, CODE header+data");

    // BASIC header.
    assert_eq!(b[0][0], 0x00, "header block flag");
    assert_eq!(b[0][1], 0, "type 0 = BASIC program");
    assert_eq!(&b[0][2..12], b"DEMO      ", "10-char name");
    assert_eq!(
        u16::from_le_bytes([b[0][14], b[0][15]]),
        10,
        "autostart line 10"
    );

    // BASIC data: line number 10 (big-endian) and a terminating ENTER.
    assert_eq!(b[1][0], 0xFF, "data block flag");
    assert_eq!(&b[1][1..3], &[0x00, 0x0A], "line number 10");
    assert_eq!(*b[1].last().unwrap(), 0x0D, "line ends with ENTER");

    // CODE header: type 3, load address 0x8000, length 4.
    assert_eq!(b[2][1], 3, "type 3 = CODE");
    assert_eq!(u16::from_le_bytes([b[2][12], b[2][13]]), 4, "code length");
    assert_eq!(
        u16::from_le_bytes([b[2][14], b[2][15]]),
        0x8000,
        "load address"
    );

    // CODE data == our bytes.
    assert_eq!(b[3][0], 0xFF, "data block flag");
    assert_eq!(&b[3][1..], &code, "code bytes round-trip");
}

#[test]
fn compile_to_tap_needs_entry() {
    assert!(rustz80::compile_to_tap("fn other() {}", "main", "X").is_err());
    assert!(rustz80::compile_to_tap("fn main() {}", "main", "X").is_ok());
}
