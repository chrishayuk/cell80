//! The nth Fibonacci number (F(0)=0, F(1)=1, F(n)=F(n-1)+F(n-2)), checked: escalates instead of silently wrapping once F(n) would exceed u32::MAX (n >= 47).
//! tags: number, fibonacci, sequence, combinatorics, checked, wide, u32, escalate
//! entry: FibonacciChecked::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if F(n) would exceed u32::MAX (n >= 47)
struct FibonacciChecked { n: u32, result: u32 }
impl FibonacciChecked {
    fn run(&mut self) -> u16 {
        let mut a = 0u32;
        let mut b = 1u32;
        let mut i = 0u32;
        while i < self.n {
            let next = a.wrapping_add(b);
            if next < b { halt(0xFF05u16); }
            a = b;
            b = next;
            i = i + 1u32;
        }
        self.result = a;
        1u16
    }
}
