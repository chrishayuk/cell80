//! Exercise the ZEX harness plumbing (`run_zex` BDOS trap + warm-boot exit) with a tiny
//! synthetic CP/M program, so the harness is validated before a real ZEXDOC/ZEXALL ROM is
//! wired in. Also covers `FlatBus::default()`.

use z80_tests::{run_zex, FlatBus};

#[test]
fn flatbus_default_is_new() {
    let bus = FlatBus::default();
    assert_eq!(bus.tstates, 0);
    assert_eq!(bus.mem[0], 0);
}

#[test]
fn run_zex_services_bdos_and_warm_boots() {
    // A CP/M .com (loaded at 0x0100) that prints a string (BDOS fn 9), then a char
    // (fn 2), then makes an unknown BDOS call (fn 1, ignored), then warm-boots (JP 0).
    #[rustfmt::skip]
    let rom = [
        0x0E, 0x09,             // LD C, 9          ; fn 9 = print $-string
        0x11, 0x17, 0x01,       // LD DE, 0x0117    ; -> "Hi$"
        0xCD, 0x05, 0x00,       // CALL 0x0005      ; BDOS
        0x0E, 0x02,             // LD C, 2          ; fn 2 = print char in E
        0x1E, 0x58,             // LD E, 'X'
        0xCD, 0x05, 0x00,       // CALL 0x0005
        0x0E, 0x01,             // LD C, 1          ; fn 1 = unknown here (ignored)
        0xCD, 0x05, 0x00,       // CALL 0x0005
        0xC3, 0x00, 0x00,       // JP 0x0000        ; warm boot -> finished
        0x48, 0x69, 0x24,       // "Hi$"            ; at 0x0117
    ];
    let out = run_zex(&rom, 10_000);
    assert_eq!(out, "HiX");
}
