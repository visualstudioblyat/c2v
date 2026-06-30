# Gap Analysis: What Production HLS Compilers Ship That c2v Does Not

This document identifies features present in production HLS tools (Vitis HLS,
Catapult, Intel HLS, Bambu) that c2v either lacks entirely or has only stub-
level support for. Focus is on gaps we have NOT already identified in prior
research documents. Organized by severity of impact on real adoption.

---

## 1  Type System Gaps

### 1.1  No Arbitrary-Precision Integer/Fixed-Point Types

Vitis HLS ships `ap_int<W>`, `ap_uint<W>`, `ap_fixed<W,I,Q,O>` with full
bit-accurate C simulation. Users write `ap_uint<13>` and get exactly 13-bit
datapaths in hardware. c2v's type system is limited to C-native widths (8, 16,
32, 64, 128). Our width-narrowing pass can trim unused bits post-analysis, but
that is the opposite workflow: the user cannot *request* 13-bit arithmetic, and
simulation does not model truncation/saturation/rounding semantics.

**What is needed:** Either a pragma system (`#pragma c2v width(x, 13)`) that
overrides inferred widths, or support for a lightweight `c2v_uint<W>` header
that behaves identically in C simulation and maps to exact widths during
synthesis. Fixed-point types with configurable quantization (truncate, round,
convergent) and overflow (wrap, saturate) modes are a separate deliverable.

### 1.2  No Floating-Point Path

Vitis 2024.2 added arbitrary-precision floating-point types and has always
supported `float`/`double` mapped to vendor FP IP cores. c2v has zero FP
support. Even if our restricted-C subset excludes it, serious HLS users
expect at least `half`/`float` mapped to DSP-based FP units or CORDIC.

### 1.3  No Enum-to-Encoding Control

Vitis HLS lets users control FSM encoding (one-hot, binary, gray) via pragmas.
c2v has `fsm_encoding.rs` but no user-facing control or automatic encoding
selection based on state-count heuristics.

---

## 2  Dataflow and Task-Level Parallelism

### 2.1  No Dataflow / Task-Level Pipeline

Vitis HLS's `#pragma HLS dataflow` is arguably its most important feature for
streaming architectures. It decomposes a function into concurrent tasks
connected by FIFOs (hls::stream) or ping-pong buffers. Each task runs as an
independent pipeline stage with its own FSM.

c2v has `streaming_accel.rs`, `fifo.rs`, `accum_stream.rs`, and
`latency_insensitive.rs` in emit -- but these are output-format modules, not
a compiler-internal dataflow decomposition. We do not:
- Automatically identify producer/consumer task boundaries
- Insert FIFO channels between tasks
- Generate per-task FSMs that run concurrently
- Support explicit `c2v_stream<T>` channel types in user code

This is the single largest architectural gap versus Vitis HLS.

### 2.2  No Function-Level Hierarchy

Vitis HLS compiles multi-function C programs, preserving the call hierarchy in
RTL (each function becomes an RTL module with its own schedule). c2v operates
on a single function. There is no support for:
- Inlining decisions (inline small functions, keep large ones as modules)
- Shared function instances (call a hardware module from multiple sites)
- Recursive function unrolling (not even bounded-depth)

---

## 3  Memory Architecture Gaps

### 3.1  No Multi-Port Memory Banking/Arbitration

c2v has `dual_port_bram.rs` and `array_partition.rs`, but production tools
generate full memory banking with N-way arbitration, bank-conflict detection,
and automatic port-assignment scheduling. Bambu's memory subsystem generates
multi-port arbitrated memory controllers with configurable banking factors.

We lack:
- Automatic bank-factor inference from access patterns
- Arbitration mux generation for bank conflicts
- Support for partitioning dimensions (cyclic, block, complete) per array axis
- Memory reshaping (packing multiple narrow elements into one wide BRAM word)

### 3.2  No Cache / Scratchpad for Irregular Access

Vitis HLS can generate cache controllers for irregular memory access patterns
(pointer-chasing, indirect indexing). We assume all arrays are directly
addressable BRAMs with known bounds. Any irregular access pattern simply fails.

### 3.3  No External Memory Interface (DDR/HBM)

Production HLS generates AXI master interfaces for off-chip memory with burst
inference, outstanding transaction tracking, and read-after-write hazard
management. c2v has no concept of external memory -- everything is on-chip BRAM.

---

## 4  Interface Protocol Gaps

### 4.1  No AXI4-Full / AXI4-Lite Register Map Generation

c2v has `axi4_lite.rs` and `axi_stream.rs` in emit, but Vitis HLS auto-
generates a full AXI4-Lite register map with software driver headers (C driver
for ARM/MicroBlaze), interrupt support, and auto-restart control registers.

Missing pieces:
- Register map with named fields mapped to function arguments
- Software driver generation (C header + source)
- Interrupt generation on ap_done / ap_ready
- AXI4-Full burst master for array arguments
- Control/status register block with ap_start/ap_done/ap_idle protocol

### 4.2  No FIFO Interface with Backpressure Handshake

While c2v emits `fifo.rs` RTL, there is no integration with scheduling -- the
scheduler does not model FIFO stalls, backpressure propagation, or credit-based
flow control. Vitis HLS's `hls::stream<T>` is deeply integrated: the scheduler
inserts pipeline stalls when a stream read would block.

### 4.3  No M_AXI Burst Inference

Vitis HLS analyzes sequential array access patterns and automatically generates
AXI burst transactions (INCR bursts with configurable max burst length). This
requires loop analysis, stride detection, and burst-length computation -- none
of which c2v performs.

---

## 5  Verification and Debug Gaps

### 5.1  No C/RTL Co-Simulation

Vitis HLS reuses the C testbench to drive RTL simulation automatically: it
wraps the generated Verilog in a cosimulation harness, feeds C stimulus, and
compares outputs cycle-by-cycle. c2v has `cocotb.rs` and `testbench_gen.rs` but
no automated flow that takes the user's C test vectors and drives them through
the generated RTL with automatic pass/fail.

### 5.2  No Formal Equivalence Checking Path

Production flows verify C-to-RTL equivalence formally. c2v has `verify/mod.rs`
and `verify/reference.rs` but no integration with formal tools (Yosys-SMTBMC,
SymbiYosys, or commercial equivalence checkers).

### 5.3  No Source-Level Debug Mapping

Vitis HLS maintains a bidirectional mapping between C source lines and RTL
signals, enabling source-level waveform debugging. c2v tracks `source_line` in
DfgNode but does not emit debug metadata that a waveform viewer can consume.
No VCD annotation, no DWARF-like debug info for hardware.

### 5.4  No ILA Probe Insertion from C Variables

c2v has `ila_probes.rs` but production tools let users mark C variables for
hardware observation. The tool then automatically inserts ILA cores, connects
probe signals, and generates trigger configurations -- all from C-level
annotations. We have no such user-facing flow.

---

## 6  Clock, Timing, and Power Gaps

### 6.1  No Multi-Clock Domain Support

All c2v output is single-clock. Production HLS tools (and academic extensions
to LegUp) support multiple clock domains with automatic CDC (clock-domain
crossing) insertion. Real SoC designs routinely need a fast datapath clock and
a slower control/AXI clock.

### 6.2  No Clock Gating / Power Optimization

Academic research shows HLS-level clock gating can reduce dynamic power with
< 0.25% area overhead. c2v performs no power optimization. No clock enable
generation for idle pipeline stages, no operand isolation, no register
enable gating based on lifetime analysis (which we already compute).

### 6.3  No Post-Place-and-Route Feedback Loop

Vitis HLS 2025.x integrates post-implementation timing data to refine
scheduling estimates. c2v's `timing_model.rs` uses static delay estimates with
no feedback from actual synthesis results. The `synth.rs` Yosys integration
runs synthesis but does not feed timing slack back into the scheduler.

---

## 7  Scheduling and Optimization Gaps

### 7.1  No Performance Pragma / Automatic Target-Frequency Scheduling

Vitis 2025 introduced `#pragma HLS performance` / `set_performance_directive`
that lets users specify target throughput and the tool automatically selects
pipeline II, unroll factors, and array partitioning. c2v requires the user to
manually configure these or uses fixed heuristics.

### 7.2  No Stencil / Sliding-Window Pragma

Vitis 2024.2 added `#pragma HLS stencil` for image/video filter kernels. This
automatically generates line buffers, window buffers, and the sliding-window
access pattern. c2v has no domain-specific pattern recognition for stencils.

### 7.3  No Automatic Loop Flattening / Loop Merging

Vitis HLS flattens nested perfect loops into a single loop to reduce FSM
overhead and enable better pipelining. It also merges consecutive loops with
the same trip count. c2v has `loop_unroll.rs` and `loop_detect.rs` but no
flattening or merging passes.

### 7.4  No Speculative Execution / Branch Prediction

Production tools speculatively execute both sides of a branch and select the
result, eliminating control-flow-dependent pipeline stalls. c2v has
`if_conversion.rs` which is related but does not handle speculative execution
with rollback for memory operations.

---

## 8  FSM Quality and Control Logic Gaps

### 8.1  FSM Bloat for Control-Heavy Code

Research shows HLS-generated FSMs can dominate area for control-heavy designs,
with timing failures on the next-state logic. Known mitigations:
- Hierarchical FSM decomposition (sub-FSMs for inner loops)
- FSM encoding optimization (one-hot for small, binary for large)
- Pipeline the FSM next-state logic itself

c2v generates FSMs but has no hierarchical decomposition or automatic encoding
selection based on state count.

### 8.2  No Speculative State Transitions

Advanced HLS tools predict the next state one cycle early, reducing the FSM
critical path. c2v does not implement this optimization.

---

## 9  Ecosystem and Tooling Gaps

### 9.1  No Tcl/Python Scripting Interface

Vitis HLS is scriptable via Tcl. Users automate DSE runs, constraint sweeps,
and batch synthesis. c2v is CLI-only with no scripting API. The `dse.rs` module
explores design space internally but is not user-extensible.

### 9.2  No Library/IP Integration

Vitis HLS ships L1 library functions (FFT, FIR, matrix multiply, vision/OpenCV
primitives, linear algebra solvers). Users instantiate these as optimized
black-box IP. c2v has no IP library mechanism -- everything must be expressed in
the input C code and synthesized from scratch.

### 9.3  No Incremental Compilation

Changing one pragma in Vitis HLS can trigger a re-synthesis of only the
affected module. c2v re-runs the entire pipeline from C to Verilog on every
invocation. For large designs this will become prohibitive.

### 9.4  No GUI / Report Viewer

Vitis HLS provides a Timeline Trace Viewer showing the runtime profile of the
synthesized design: which operations execute in which cycle, pipeline
utilization, FIFO depths. c2v's `report.rs` emits text reports. No interactive
visualization of schedule, binding, or resource utilization.

---

## 10  Summary: Priority-Ordered Gap List

| Priority | Gap | Impact | Effort |
|----------|-----|--------|--------|
| P0 | Dataflow / task-level parallelism | Blocks streaming designs | Large |
| P0 | Multi-function hierarchy | Blocks real programs | Large |
| P0 | Arbitrary-precision types | Blocks optimal area | Medium |
| P1 | AXI4-Lite register map + driver gen | Blocks SoC integration | Medium |
| P1 | C/RTL co-simulation flow | Blocks trust in output | Medium |
| P1 | Multi-port memory banking | Blocks memory-heavy designs | Medium |
| P1 | M_AXI burst inference | Blocks external memory | Medium |
| P2 | Performance pragma / auto-tuning | Usability gap | Medium |
| P2 | Loop flattening / merging | QoR gap | Small |
| P2 | Clock gating / power optimization | Power-sensitive designs | Medium |
| P2 | Floating-point support | Blocks FP-heavy designs | Large |
| P2 | Multi-clock domain support | Blocks SoC integration | Large |
| P3 | Stencil pragma | Domain-specific usability | Small |
| P3 | Formal equivalence checking | Verification confidence | Medium |
| P3 | Source-level debug mapping | Debug usability | Medium |
| P3 | IP library / black-box instantiation | Ecosystem gap | Medium |
| P3 | Scripting interface | Automation gap | Small |

---

## Sources

- [Vitis HLS Product Page](https://www.amd.com/en/products/software/adaptive-socs-and-fpgas/vitis/vitis-hls.html)
- [Vitis What's New](https://www.amd.com/en/products/software/adaptive-socs-and-fpgas/vitis/vitis-whats-new.html)
- [Vitis HLS User Guide UG1399](https://www.xilinx.com/support/documents/sw_manuals/xilinx2022_2/ug1399-vitis-hls.pdf)
- [Survey and Evaluation of FPGA HLS Tools](https://janders.eecg.utoronto.ca/pdfs/tcad_hls.pdf)
- [Bambu HLS Framework (IEEE)](https://ieeexplore.ieee.org/document/9586110/)
- [HLS State Machine Problems (Xilinx Forums)](https://forums.xilinx.com/t5/High-Level-Synthesis-HLS/HLS-State-Machine-Problems/td-p/639638)
- [HLS Memory Arbitration (Wickerson)](https://johnwickerson.github.io/papers/easy_tc21.pdf)
- [Multi-Clock HLS (IEEE)](https://ieeexplore.ieee.org/document/8457641/)
- [Clock Gating in HLS (MDPI)](https://www.mdpi.com/2079-9292/10/1/73)
- [HLS Debug Observability (ResearchGate)](https://www.researchgate.net/publication/323502625)
- [Xilinx HLS Arbitrary Precision Types (GitHub)](https://github.com/Xilinx/HLS_arbitrary_Precision_Types)
- [HLS FPGA Design Guide (Siemens HLS Academy)](https://hls.academy/topics/hls-fpga/)
- [FPGA Design Flow (HardwareBee)](https://hardwarebee.com/ultimate-guide-fpga-design-flow/)
- [Stream-HLS Automatic Dataflow Acceleration](https://arxiv.org/html/2501.09118v1)
- [TAPA Task-Parallel Dataflow (arxiv)](https://arxiv.org/pdf/2209.02663)
- [Reliability of HLS Tools (Wickerson)](https://johnwickerson.github.io/papers/fuzzingHLS.pdf)
