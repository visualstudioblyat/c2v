# c2v

C-to-Verilog HLS compiler. Restricted C in, synthesizable Verilog out. Zero pragmas, 20ms compile, verified correct.

## Quick start

```bash
cargo build --release
c2v=./target/release/c2v

# Compile and verify
$c2v examples/fd_hash_round.c -f fd_hash_round --verify

# Yosys synthesis (real gate counts)
$c2v examples/fd_hash_round.c -f fd_hash_round --verify --synth

# Pipelined output (II=1, new result every clock)
$c2v examples/fd_add.c -f fd_add_compute --pipeline --verify

# Auto loop pipelining (>16 iterations auto-detected)
$c2v examples/fd_fibonacci.c -f fd_fibonacci

# Design space exploration (find Pareto-optimal config)
$c2v examples/fd_hash_round.c -f fd_hash_round --dse 100 --verify

# Streaming BRAM accelerator
$c2v examples/fd_stream_sum.c -f fd_element_process --stream "input:output"

# Accumulator with forwarding
$c2v examples/fd_accum_sum.c -f fd_accum_add --accum-stream "data:result"

# BRAM-ported accelerator wrapper
$c2v examples/fd_add.c -f fd_add_compute --accel "a:+result"

# Bjorklund non-integer II (novel — no other HLS tool)
$c2v examples/fd_add.c -f fd_add_compute --loop-pipeline --bjorklund-ii 3,8

# Dataflow: multi-function FIFO-connected pipeline
$c2v examples/fd_two_stage.c -f fd_two_stage --dataflow "fd_stage1,fd_stage2"

# Target FPGA
$c2v examples/fd_hash_round.c -f fd_hash_round -T xc7a35t --synth
```

## What it compiles

Restricted C — no heap, no recursion, no function pointers, no floating point.

**Supported:** uint/int (8/16/32/64-bit, signed), arrays, POD structs, nested structs, pointer-to-struct params, bounded for-loops (auto unroll ≤16, auto pipeline >16), if/else cascades, switch/case, ternary, function inlining, compound assignment, enum constants, sizeof.

## Emit modes

| Mode | Flag | Description |
|------|------|-------------|
| FSM | (default) | Single-invocation compute module |
| Pipeline | `--pipeline` | II=1 streaming datapath |
| Loop pipeline | auto (>16 iters) | Pipelined loop with iteration counter |
| Bjorklund | `--bjorklund-ii k,n` | Non-integer II (novel) |
| BRAM accelerator | `--accel "in:+out"` | Read/compute/write with core instantiation |
| Streaming | `--stream "in:out"` | II=1 pipelined BRAM read→compute→write |
| Accumulator | `--accum-stream "in:out"` | II=1 with forwarded accumulator |
| Dataflow | `--dataflow "f1,f2"` | Multi-function FIFO-connected pipeline |

## Verification

1000 random test vectors through C reference model and generated Verilog (iverilog). Streaming accelerator also verified end-to-end in simulation (BRAM fill→process→check).

## Optimization

- **24 optimization passes:** strength reduce, const fold, reassociate, identity elim, mux simplify, CSE, constant prop, loop invariant, chain combine, duplicate store, dead node, copy prop, operator merge, share mux, retiming, loop unroll, width narrow, plus second const fold + dead node
- **Tree balancing:** always-on, reduces associative chains O(n) → O(log n)
- **Combinational chaining:** AND/OR/XOR/NOT/Mux/comparisons as wire+assign
- **DSP48 inference:** multiply-accumulate patterns get `(* use_dsp = "yes" *)`
- **Build-time constant folding:** 3+5 → 8 during parsing
- **Design space exploration:** 1600 points/sec Pareto frontier search

## Novel features (no other HLS tool has these)

- **Bjorklund non-integer II** — maximally-even pipeline schedule from nuclear physics (Bjorklund 2003, Toussaint 2005). E(3,8) = [10010010], effective II=2.67 vs Vitis integer II=3.
- **Bounded treewidth scheduling** — Thorup (1998) proves goto-free C has treewidth ≤ 6, making scheduling polynomial (FPT) instead of NP-hard.
- **Shannon entropy parallelism prediction** — predicts achievable speedup before scheduling runs.
- **Tropical max-plus eigenvalue** — Howard's algorithm computes minimum II in O(n²) vs LP O(n³).

## Synthesis results (Yosys synth_xilinx)

| Design | Cycles | LUTs | FFs | DSPs |
|--------|--------|------|-----|------|
| fd_add_compute | 3 | 69 | 133 | 0 |
| fd_hash_round (SHA-256) | 8 | 323 | 410 | 0 |
| fd_pipe_mac | 7 | 73 | 139 | 0 |
| fd_det2x2 | 6 | 40 | 106 | 0 |
| fd_chacha_quarter_round | 15 | 17 | 17 | 0 |
| fd_crc32_byte | 27 | 165 | 309 | 0 |
| fd_aes_mix_column | 13 | — | — | 0 |
| fd_ip_checksum | 9 | 27 | — | 0 |
| fd_packet_parser | 8 | 24 | 37 | 0 |
| fd_rgb565_pack | 7 | 9 | 63 | 0 |
| fd_alpha_blend | 12 | 134 | — | 6 |

## Target FPGAs

13 devices across 3 vendors: Xilinx (Artix-7, Kintex-7, UltraScale+, Alveo), Intel (Cyclone V, Arria 10, Agilex), Lattice (iCE40, ECP5, CrossLink-NX).

## Interface wrappers

AXI-Stream, AXI4-Lite slave, Avalon-ST, Wishbone B4, CSR register block, FIFO, ILA debug probes, Vivado/Quartus project TCL.

## Research

6 research files (6,000+ lines) with cited mathematical foundations: scheduling theory, biological algorithms, loop pipelining, cross-domain connections, competing tools analysis, gap analysis vs Vitis HLS.

## Stats

- **16,600+ lines** of Rust
- **161 tests**, 0 failures
- **149 examples** across crypto, networking, DSP, math, control, pixel processing
- **77 commits**
- Compiles in **~20ms** (vs Vitis 15+ minutes)

## Requirements

- Rust 1.76+
- Icarus Verilog (`iverilog`) for verification
- Yosys (optional) for synthesis
