//! Excel COUNTBLANK(range): counts the blank (truly empty) cells in a range -- the mirror-image count from COUNT (which only counts cells holding a NUMBER) and COUNTA (which counts every NON-empty cell, numeric or not): COUNTBLANK counts the cells with no content at all. Because a blank cell carries no numeric content, this cell does NOT take a values array like excel_npv's cash-flow envelope -- blank-ness is resolved upstream by the host (same division of labor every array cell in this family uses for its inputs) and arrives here as a bitmask: blank_mask has bit i set exactly when range slot i (of up to 16) is blank. count names how many of the 16 envelope slots are actually part of the caller's range (the array-state envelope wall -- Excel's real COUNTBLANK takes a range of any size, but this dialect's fixed-size state field caps one call at 16 cells, exactly like excel_npv's 16-slot cash-flow array). The answer is popcount(blank_mask & low_count_mask), where low_count_mask = (1 << count) - 1 clears every mask bit at or beyond the caller's actual arity before counting, so stale/undefined bits past `count` can never be mistaken for a live blank slot. This mirrors the bit-mask pack's mask_overlap_count(a, b) = popcount(a & b), but cells cannot call each other, so its popcount-of-AND technique is inlined directly here rather than invoked.
//! tags: excel, countblank, count-blank, blank, empty, empty-cells, blank-cells, range, bitmask, mask, popcount, array, math-stat
//! entry: ExcelCountBlank::run
//! limits: fixed 16-slot range envelope, not caller-configurable (the array-state envelope wall, same limitation excel_npv documents for its cash-flow array); escalates (halt 0xFF06, out_of_domain) if count is 0 or exceeds 16; any blank_mask bit at or beyond position `count` is masked off before counting, so it can never be mistaken for a live blank slot in the caller's actual range.
struct ExcelCountBlank {
    blank_mask: u16,
    count: u16,
    blanks: u16,
}
impl ExcelCountBlank {
    fn run(&mut self) -> u16 {
        if self.count == 0u16 { halt(0xFF06u16); }
        if self.count > 16u16 { halt(0xFF06u16); }

        // low_count_mask = (1 << count) - 1, built by doubling one bit at a time
        // (the dialect only allows a literal shift amount on a u32 LHS -- `1u32 <<
        // count` doesn't lower -- so a variable-count shift has to be a loop of
        // literal `<< 1u32` steps instead) rather than a single variable-width
        // shift, then narrowed back down to u16 to AND against blank_mask --
        // mask_overlap_count's own popcount of a bitwise AND, inlined here (cells
        // can't call each other) rather than calling that cell directly.
        let mut low_count_mask32 = 0u32;
        let mut k = 0u16;
        while k < self.count {
            low_count_mask32 = (low_count_mask32 << 1u32) | 1u32;
            k = k + 1u16;
        }
        let low_count_mask = low_count_mask32 as u16;

        let masked = self.blank_mask & low_count_mask;

        let mut v = masked;
        let mut c = 0u16;
        while v != 0u16 {
            c = c + (v & 1u16);
            v = v >> 1u16;
        }
        self.blanks = c;
        1u16
    }
}
