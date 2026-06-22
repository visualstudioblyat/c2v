# c2v

C to Verilog compiler. Takes restricted C (no pointers, no malloc, no recursion) and emits verified, synthesizable Verilog.

## Quick start

```bash
cargo build --release

# Compile a C function to Verilog, verify correctness
./target/release/c2v examples/fd_hash_round.c -f fd_hash_round --verify

# With tree balancing optimization
./target/release/c2v examples/fd_fir8.c -f fd_fir8_compute --verify --balance

# Target a specific FPGA
./target/release/c2v examples/fd_hash_round.c -f fd_hash_round -T xc7a35t --synth

# Run Yosys synthesis for real gate counts
./target/release/c2v examples/fd_hash_round.c -f fd_hash_round --synth --balance

# Multi-file compilation
./target/release/c2v examples/fd_helpers.c examples/fd_multifile.c -f fd_blend --verify

# With preprocessor (for #include and macros)
./target/release/c2v examples/fd_with_header.c -f fd_compute --verify --preprocess
```

## What it compiles

Restricted C subset — no pointers, no dynamic allocation, no recursion, no floating point.

**Supported:** scalars (u8-u64, signed), arrays, POD structs, nested structs, bounded for-loops, if/else, switch/case, ternary, function inlining (3+ levels), compound assignment (+=, ^=), bitwise NOT, multi-file.

**Rejected with error:** malloc, free, goto, function pointers, while loops, varargs, VLAs, stdlib includes.

## Verification

Every compile runs 1000 random test vectors through both the C reference model and the generated Verilog (via Icarus Verilog). The compiler refuses to emit if any mismatch is found.

```
[verify]    1000/1000 correct       ✓  52ms
```

## Optimization

- **SDC scheduling** with HiGHS LP solver
- **Resource constraints:** `--max-mul 2` limits multipliers, trades latency for area
- **Tree balancing:** `--balance` restructures serial reductions into parallel trees
- **12 optimization passes:** strength reduce, const fold, CSE, identity elim, mux simplify, reassociate, loop invariant, chain combine, duplicate store elim, dead node, width narrow

## Target FPGAs

13 devices across 3 vendors:

| Vendor | Devices |
|--------|---------|
| Xilinx | Artix-7, Kintex-7, UltraScale+, Alveo VU9P |
| Intel | Cyclone V, Arria 10, Agilex |
| Lattice | iCE40 UP5K/HX8K, ECP5 25F/85F, CrossLink-NX |

```bash
./target/release/c2v --list-targets
```

## Integration

- **AXI-Stream wrapper:** plugs into Vivado block design
- **Vivado project TCL:** one command from C to bitstream
- **Quartus project TCL:** Intel FPGA support
- **Yosys synthesis:** real gate counts via `--synth`
- **VCD waveform:** `--wave` for GTKWave debugging
- **ML decision trace:** `--trace` emits JSON for ML-guided optimization

## Examples

52 verified examples including SHA-256 (1+4 round), AES (S-box + full round), CORDIC, CRC-8/32, FIR/IIR filters, PID controller, LFSR, matrix multiply, sorting network, UART receiver, SPI master, I2C master, Huffman decoder, ReLU neural network layer, PWM generator, priority encoder, and more.

## Requirements

- Rust 1.76+
- Icarus Verilog (`iverilog`) for verification
- Yosys (optional) for synthesis

## License

Apache 2.0
