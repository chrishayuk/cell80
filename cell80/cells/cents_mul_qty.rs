//! Total price in cents (the minor unit of any decimal currency — cents, pence, kopecks, not USD specifically): unit_cents * qty. Escalates (needs_wider_math) on multiply overflow — distinct from mul_u16_u16_to_u32 (that one always fits u32 exactly; this one's unit_cents is already wide and can genuinely overflow).
//! tags: money, cents, price, multiply, quantity, checked, wide, u32
//! entry: CentsMulQty::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if unit_cents * qty overflows u32
struct CentsMulQty { unit_cents: u32, qty: u16, total: u32 }
impl CentsMulQty {
    fn run(&mut self) -> u16 {
        let q = self.qty as u32;
        let product = self.unit_cents.wrapping_mul(q);
        if self.unit_cents != 0u32 && product / self.unit_cents != q { halt(0xFF05u16); }
        self.total = product;
        1u16
    }
}
