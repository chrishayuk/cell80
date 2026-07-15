//! Generates the interpreter kernel's dialect-specific source (MSL, CUDA) from
//! the shared bytecode opcode constants ([`super::bytecode`]) and one shared
//! kernel body, so the kernel decoder and the bytecode encoder cannot drift —
//! an opcode-semantics fix lands in both dialects by construction.

use super::bytecode::{
    MAX_LOCALS, OP_BIN, OP_BIN32, OP_CLZ, OP_CMP, OP_CMP32, OP_CTZ, OP_DUP, OP_HALT, OP_JMP,
    OP_JMPZERO, OP_POP, OP_POPCNT, OP_PUSHLIT, OP_PUSHVAR, OP_RET, OP_SEXTHI, OP_SHIFT32,
    OP_SHIFTLIT, OP_STEP, OP_STORE, OP_TRUNC,
};
use super::STACK_CAP;

/// The decoder's constant block, generated from the Rust-side opcode/status/
/// budget constants so the kernel and [`super::bytecode`]'s encoder cannot
/// drift. `decl` is the dialect's constant-declaration prefix.
fn interp_const_block(decl: &str) -> String {
    format!(
        "{decl} OP_STEP={OP_STEP},OP_PUSHLIT={OP_PUSHLIT},OP_PUSHVAR={OP_PUSHVAR},OP_BIN={OP_BIN},OP_SHIFTLIT={OP_SHIFTLIT},\n\
         \x20             OP_CMP={OP_CMP},OP_TRUNC={OP_TRUNC},OP_STORE={OP_STORE},OP_POP={OP_POP},OP_JMPZERO={OP_JMPZERO},OP_JMP={OP_JMP},OP_RET={OP_RET},OP_DUP={OP_DUP},OP_HALT={OP_HALT},\n\
         \x20             OP_POPCNT={OP_POPCNT},OP_CLZ={OP_CLZ},OP_CTZ={OP_CTZ},OP_BIN32={OP_BIN32},OP_SHIFT32={OP_SHIFT32},OP_CMP32={OP_CMP32},OP_SEXTHI={OP_SEXTHI};\n\
         {decl} ST_OK={ok}u, ST_DIV0={div0}u, ST_HALT={halt}u, ST_FUEL={fuel}u;\n\
         {decl} FUEL={budget}u;\n\
         #define MAX_LOCALS {MAX_LOCALS}\n\
         #define MAX_STACK  {STACK_CAP}\n",
        ok = crate::STATUS_OK,
        div0 = crate::STATUS_DIV0,
        halt = crate::STATUS_HALT,
        fuel = crate::STATUS_FUEL,
        budget = crate::FUEL,
    )
}

/// The interpreter kernel's MSL source: the Metal header + the generated
/// decoder-constant block + the `[[buffer]]`-bound signature over the shared
/// [`KERNEL_BODY`]. The bytes are golden-locked (`tests/codegen_snapshot.rs`).
pub fn interp_source_msl() -> String {
    format!(
        "\n\
         #include <metal_stdlib>\n\
         using namespace metal;\n\
         \n\
         {consts}\
         \n\
         kernel void interp(\n\
         \x20   const device uint*   code       [[buffer(0)]],\n\
         \x20   const device uint*   cell_table  [[buffer(1)]],\n\
         \x20   const device ushort* probes     [[buffer(2)]],\n\
         \x20   device ushort*       out        [[buffer(3)]],\n\
         \x20   constant uint&       n_probes    [[buffer(4)]],\n\
         \x20   uint cell [[threadgroup_position_in_grid]],\n\
         \x20   uint p    [[thread_position_in_threadgroup]])\n\
         {{\n\
         {KERNEL_BODY}",
        consts = interp_const_block("constant uint"),
    )
}

/// The interpreter kernel's CUDA source: typedefs + wrappers giving the
/// shared body its MSL vocabulary (`popcount`/`clz`/`ctz` over
/// `__popc`/`__clz`/`__ffs`; `min` as a macro so it can neither collide with
/// a builtin nor be missing), the generated decoder-constant block, and the
/// `__global__` signature with the one-block-per-cell / probes-across-lanes
/// launch shape. `__clz(0) == 32`, so the body's unguarded `clz(x)-16u` trick
/// transfers; the body guards `ctz`'s zero case itself.
pub fn interp_source_cuda() -> String {
    format!(
        "\n\
         typedef unsigned int uint;\n\
         typedef unsigned short ushort;\n\
         \n\
         #define min(a, b) (((a) < (b)) ? (a) : (b))\n\
         static __device__ uint popcount(uint x) {{ return (uint)__popc((int)x); }}\n\
         static __device__ uint clz(uint x) {{ return (uint)__clz((int)x); }}\n\
         static __device__ uint ctz(uint x) {{ return (uint)(__ffs((int)x) - 1); }}\n\
         \n\
         {consts}\
         \n\
         extern \"C\" __global__ void interp(\n\
         \x20   const uint* __restrict__ code,\n\
         \x20   const uint* __restrict__ cell_table,\n\
         \x20   const ushort* __restrict__ probes,\n\
         \x20   ushort* out,\n\
         \x20   uint n_probes)\n\
         {{\n\
         \x20   uint cell = blockIdx.x;\n\
         \x20   uint p = threadIdx.x;\n\
         {KERNEL_BODY}",
        consts = interp_const_block("constexpr uint"),
    )
}

/// The interpreter kernel's shared body — everything after the per-dialect
/// signature. Pure C over the dialect header's typedef/intrinsic vocabulary
/// (`uint`/`ushort`, `popcount`/`clz`/`ctz`/`min`), byte-identical across
/// dialects so an opcode-semantics fix lands in both by construction. Note
/// div is a `switch` arm over the operand stack — the compiled backend's
/// noinline-helper dodge was never needed here.
const KERNEL_BODY: &str = r#"    if (p >= n_probes) return;
    uint code_off = cell_table[cell*3+0];
    uint n_locals = cell_table[cell*3+1];
    uint params   = cell_table[cell*3+2];

    ushort slots[MAX_LOCALS];
    for (uint i=0;i<MAX_LOCALS;i++) slots[i]=0;
    for (uint i=0;i<params && i<3u;i++) slots[i]=probes[p*3+i];

    ushort stack[MAX_STACK];
    int sp=0;
    uint steps=0u, status=ST_OK, pc=code_off, guard=0u;
    ushort r0=0,r1=0,r2=0;
    bool done=false;
    while(!done){
        if(++guard > 400000000u){ status=ST_FUEL; break; }
        uint op = code[pc*2]; uint arg = code[pc*2+1];
        switch(op){
          case OP_STEP: steps+=arg; if(steps>=FUEL){status=ST_FUEL;done=true;} break;
          case OP_PUSHLIT: stack[sp++]=(ushort)(arg & 0xFFFFu); break;
          case OP_PUSHVAR: stack[sp++]=slots[arg]; break;
          case OP_BIN: {
             ushort b=stack[--sp]; ushort a=stack[--sp];
             uint binop=arg&0xFFu; uint w=(arg>>8)&0xFFu; bool sw=(w==2u);
             ushort res=0;
             switch(binop){
               case 0: res=a+b; break;
               case 1: res=a-b; break;
               case 2: res=a*b; break;
               case 5: res=a|b; break;
               case 6: res=a&b; break;
               case 7: res=a^b; break;
               case 3: case 4: {
                  if(b==0u){ status=ST_DIV0; done=true; res=0; }
                  else if(sw){ short sa=(short)a, sb=(short)b; res=(binop==3u)?(ushort)(sa/sb):(ushort)(sa%sb); }
                  else { res=(binop==3u)?(a/b):(a%b); }
                  break;
               }
             }
             if(w==0u) res&=0xFFu;
             stack[sp++]=res; break;
          }
          case OP_SHIFTLIT: {
             ushort a=stack[--sp];
             uint k=arg&0xFFFFu; bool left=((arg>>16)&1u)!=0u; bool sgn=((arg>>17)&1u)!=0u; uint w=(arg>>18)&0x3u;
             ushort res;
             if(left){ res=(k>=16u)?0:(ushort)(a<<k); }
             else if(sgn){ short sa=(short)a; uint kk=min(k,15u); res=(ushort)(sa>>kk); }
             else { res=(k>=16u)?0:(ushort)(a>>k); }
             if(w==0u) res&=0xFFu;
             stack[sp++]=res; break;
          }
          case OP_CMP: {
             ushort b=stack[--sp]; ushort a=stack[--sp];
             uint cmp=arg&0xFFu; bool sgn=((arg>>8)&1u)!=0u; bool r;
             if(sgn && cmp<4u){ short sa=(short)a, sb=(short)b;
                switch(cmp){case 0:r=sa<sb;break;case 1:r=sa<=sb;break;case 2:r=sa>sb;break;default:r=sa>=sb;break;}
             } else {
                switch(cmp){case 0:r=a<b;break;case 1:r=a<=b;break;case 2:r=a>b;break;case 3:r=a>=b;break;case 4:r=a==b;break;default:r=a!=b;break;}
             }
             stack[sp++]=r?1:0; break;
          }
          case OP_TRUNC: { ushort a=stack[--sp]; stack[sp++]=a&0xFFu; break; }
          // u16 bit intrinsics: popcount is width-agnostic on the zero-extended
          // value; clz/ctz must be forced to the 16-bit answer (uint clz is +16;
          // uint ctz(0) is 32, but u16 wants 16).
          case OP_POPCNT: { uint x=(uint)stack[--sp]; stack[sp++]=(ushort)popcount(x); break; }
          case OP_CLZ:    { uint x=(uint)stack[--sp]; stack[sp++]=(ushort)(clz(x)-16u); break; }
          case OP_CTZ:    { uint x=(uint)stack[--sp]; stack[sp++]=(ushort)(x==0u?16u:ctz(x)); break; }
          // 32-bit ops: a u32 is two stack entries (low, then high on top).
          case OP_BIN32: {
             uint bh=stack[--sp], bl=stack[--sp], ah=stack[--sp], al=stack[--sp];
             uint a=al|(ah<<16), b=bl|(bh<<16);
             uint binop=arg&0xFFu; bool sg=((arg>>8)&1u)!=0u; uint res=0u;
             switch(binop){
               case 0: res=a+b; break;
               case 1: res=a-b; break;
               case 2: res=a*b; break;
               case 5: res=a|b; break;
               case 6: res=a&b; break;
               case 7: res=a^b; break;
               case 3: case 4: {
                  if(b==0u){ status=ST_DIV0; done=true; res=0u; }
                  else if(sg){
                     // guard MIN/-1 — 32-bit int div overflows (C UB), unlike 16-bit
                     bool ov=(a==0x80000000u && b==0xFFFFFFFFu);
                     if(binop==3u) res=ov?a:(uint)((int)a/(int)b);
                     else res=ov?0u:(uint)((int)a%(int)b);
                  } else { res=(binop==3u)?(a/b):(a%b); }
                  break;
               }
             }
             stack[sp++]=(ushort)(res&0xFFFFu); stack[sp++]=(ushort)(res>>16); break;
          }
          case OP_SHIFT32: {
             uint ah=stack[--sp], al=stack[--sp]; uint a=al|(ah<<16);
             uint k=arg&0xFFu; bool left=((arg>>16)&1u)!=0u; bool sg=((arg>>17)&1u)!=0u; uint res;
             if(sg && !left){ int sa=(int)a; uint kk=min(k,31u); res=(uint)(sa>>kk); }
             else if(k>=32u){ res=0u; }
             else if(left){ res=a<<k; }
             else { res=a>>k; }
             stack[sp++]=(ushort)(res&0xFFFFu); stack[sp++]=(ushort)(res>>16); break;
          }
          case OP_CMP32: {
             uint bh=stack[--sp], bl=stack[--sp], ah=stack[--sp], al=stack[--sp];
             uint a=al|(ah<<16), b=bl|(bh<<16);
             uint cmp=arg&0xFFu; bool sg=((arg>>8)&1u)!=0u; bool r;
             if(sg && cmp<4u){ int sa=(int)a, sb=(int)b;
                switch(cmp){case 0:r=sa<sb;break;case 1:r=sa<=sb;break;case 2:r=sa>sb;break;default:r=sa>=sb;break;}
             } else {
                switch(cmp){case 0:r=a<b;break;case 1:r=a<=b;break;case 2:r=a>b;break;case 3:r=a>=b;break;case 4:r=a==b;break;default:r=a!=b;break;}
             }
             stack[sp++]=r?1:0; break;
          }
          case OP_SEXTHI: {
             ushort lo=stack[--sp]; stack[sp++]=lo;
             stack[sp++]=((lo&0x8000u)!=0u)?(ushort)0xFFFFu:(ushort)0u; break;
          }
          case OP_STORE: slots[arg]=stack[--sp]; break;
          case OP_POP: --sp; break;
          case OP_DUP: { ushort v=stack[sp-1]; stack[sp++]=v; break; }
          case OP_JMPZERO: { ushort v=stack[--sp]; if(v==0){ pc=code_off+arg; continue; } break; }
          case OP_JMP: pc=code_off+arg; continue;
          case OP_RET: {
             uint arity=arg;
             if(arity>=1u) r0=stack[0];
             if(arity>=2u) r1=stack[1];
             if(arity>=3u) r2=stack[2];
             done=true; break;
          }
          case OP_HALT: { r0=stack[sp-1]; status=ST_HALT; done=true; break; }
          default: done=true; break;
        }
        pc++;
    }
    uint base=(cell*n_probes+p)*6u;
    out[base+0]=r0; out[base+1]=r1; out[base+2]=r2;
    out[base+3]=(ushort)status;
    out[base+4]=(ushort)(steps&0xFFFFu);
    out[base+5]=(ushort)(steps>>16);
}
"#;
