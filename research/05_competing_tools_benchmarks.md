# Competitive Landscape of HLS Tools and Benchmark Methodology for c2v

## Context and Motivation

c2v compiles restricted C (no heap, no recursion, fixed-size arrays, no pointer
aliasing) to synthesisable Verilog. It uses SDC scheduling, graph-coloring
binding, and targets both Xilinx and Intel FPGAs. ~15k lines of Rust. No LLVM
dependency. The strategic goal is to **beat Xilinx Vitis HLS** on quality of
results (area, latency, Fmax) for the restricted-C subset we support.

This document surveys every serious competitor in the HLS space, catalogues
benchmark suites with published numbers, analyses DSP inference quality across
tools, and identifies the 2024--2026 research frontier. The goal is to build a
rigorous benchmark methodology that lets us prove c2v superiority on our
restricted subset.

---

## 1  Vitis HLS --- The Target to Beat

### 1.1  Architecture Overview

Vitis HLS (formerly Vivado HLS, formerly AutoPilot/AutoESL) is AMD/Xilinx's
flagship HLS tool. Its lineage traces directly to the xPilot system developed at
UCLA by Jason Cong and Zhiru Zhang, which was licensed to AutoESL Design
Technologies and acquired by Xilinx in 2011. The SDC scheduling algorithm from
Cong & Zhang (DAC 2006) --- inducted into the FPGA and Reconfigurable Computing
Hall of Fame in 2022 --- remains at its core.

**Frontend.** Clang/LLVM-based. The frontend parses C/C++ to LLVM IR. Xilinx
partially open-sourced the frontend in 2021 (github.com/Xilinx/HLS), but the
backend remains proprietary. The open-source portion allows custom LLVM IR
transformations and new pragma definitions, but scheduling, binding, and RTL
generation are closed.

**Scheduling.** The backend uses SDC-based scheduling (the Cong--Zhang
formulation) with extensions for resource-constrained and modulo scheduling.
Specific details are proprietary, but the tool supports ASAP/ALAP analysis,
list scheduling heuristics for initial solutions, and LP-based refinement. The
SDC formulation encodes data dependencies, resource constraints, and timing
constraints as difference inequalities and solves them in polynomial time via
LP (Cong & Zhang, DAC 2006).

**Binding.** Register and functional-unit binding with resource sharing. The
specific algorithm is proprietary, but the tool performs clique partitioning and
compatibility-graph-based sharing.

**RTL Generation.** Produces hierarchical Verilog/VHDL with control FSMs. During
synthesis, hierarchical tree structures representing the generated RTL modules
are constructed, with a one-to-one mapping table between C++ functions/loops and
RTL signals (used for cosimulation and debugging).

### 1.2  Known Weaknesses

**Pragma dependency.** Vitis HLS performance is critically dependent on user-
supplied pragmas: `#pragma HLS pipeline`, `#pragma HLS unroll`,
`#pragma HLS array_partition`, `#pragma HLS inline`. Without pragmas, the tool
produces naive, sequential designs. Vitis 2024.2 introduced automatic pragma
inference for unroll, pipeline, array partition, and inline --- but this is
limited to obvious cases and cannot match expert-level pragma tuning.

**Area bloat from control logic.** The ap_ctrl_hs and ap_ctrl_chain handshake
protocols insert FSM logic, start/done/idle/ready signals, and interface
adapters around every function. In benchmarks of compute kernels, Vitis HLS
designs consume ~30% more LUTs than equivalent hand-written RTL (arxiv
2509.08067, comparing Montgomery multiplier implementations). The control
overhead is proportionally worse for small kernels.

**Compile time.** For large or highly optimized designs, Vitis HLS synthesis
takes hours. Particle-based jet-tagging networks with Vitis HLS + Vivado 2023.2
required ~17 hours for synthesis. Each design point in AutoDSE experiments
required 3--5 hours with the Vitis 2020.1 toolchain on U250. By contrast,
equivalent designs generated directly in Verilog synthesize in ~26 minutes with
10 GB peak memory vs. ~70 GB for the HLS path (arxiv 2302.06751).

**Function inlining complexity.** While inlining can reduce latency and improve
resource sharing, it complicates accurate C-to-RTL mapping. The HLS-generated
LLVM IR contains undefined functions and missing intrinsics that must be defined
or re-implemented on-the-fly (arxiv 2504.03879).

**Inaccurate resource estimates.** Vivado HLS post-synthesis resource estimates
are known to be inaccurate compared to actual post-route results. This forces
designers into slow iterate-synthesize-verify loops.

### 1.3  What c2v Can Skip

Because c2v targets restricted C with no heap, no recursion, no aliasing, and
fixed arrays, we can eliminate:

| Vitis overhead                     | c2v advantage                              |
|------------------------------------|--------------------------------------------|
| ap_ctrl_hs/chain FSM + interfaces  | Direct register-file I/O, no handshake     |
| Pointer analysis / alias analysis  | Statically known memory access patterns    |
| Dynamic memory allocation support  | All arrays are fixed, trivially partitioned|
| Recursion handling (stack frames)   | Not supported = no overhead                |
| General LLVM IR bloat              | Direct C AST to CDFG, no LLVM             |
| Pragma inference heuristics        | Compiler decides automatically             |
| Multi-hour compile times           | Target: seconds for typical kernels        |

---

## 2  LegUp / SmartHLS (Microchip)

### 2.1  History and Architecture

LegUp was developed at the University of Toronto by Andrew Canis, Jongsok Choi,
Mark Aldham, Victor Zhang, Ahmed Kammoona, Jason Anderson, Stephen Brown, and
Tomasz Czajkowski. Published at FPGA 2011 and subsequently refined through
multiple papers. Microchip acquired the technology and rebranded it as SmartHLS,
targeting their PolarFire SoC FPGAs.

LegUp was LLVM-based (like Vitis) and supported both pure-HLS and hybrid
processor/accelerator flows where a MIPS soft processor ran software portions
while hardware accelerators handled compute-intensive loops.

### 2.2  SDC Scheduling --- The Canis Contribution

LegUp's scheduling is based on the SDC formulation from Cong & Zhang (DAC 2006)
with significant extensions by Canis et al.:

- **SDC modulo scheduling** (Canis, Brown, Anderson, FPL 2014): Formulates
  modulo scheduling for pipelined loops as an SDC problem with recurrence
  minimization. Key result: over **100x improvement in runtime** over ILP-based
  modulo scheduling while attaining **optimal latency**.

- **Backtracking SDC modulo scheduling**: Achieves **32% geomean wall-clock time
  reduction** versus a state-of-the-art commercial tool and **29%** versus
  greedy approaches on loop pipelining benchmarks.

### 2.3  What LegUp/SmartHLS Does Better Than Vitis

- **Open formulation**: The SDC scheduling approach is well-documented in the
  academic literature, allowing reproducibility and extension.
- **Modulo scheduling quality**: The recurrence-minimization technique in
  Canis et al. (FPL 2014) specifically targets the initiation interval (II)
  bottleneck in pipelined loops --- a weakness in Vitis's more conservative
  approach.
- **Tighter integration with LP solvers**: LegUp uses the SDC formulation
  directly as an LP, avoiding the overhead of ILP solvers for the common case.

### 2.4  Relevance to c2v

c2v already uses SDC scheduling. The Canis modulo SDC extensions are directly
applicable to c2v's loop pipelining. Our restricted-C subset (no recursion, no
aliasing) means our SDC formulations have fewer variables and tighter bounds
than LegUp's general-purpose formulation --- we should be able to solve them
faster and find better optima.

---

## 3  Bambu (PandA Framework, Politecnico di Milano)

### 3.1  Overview

Bambu is an open-source semi-automatic HLS framework developed at DEIB,
Politecnico di Milano, by Fabrizio Ferrandi, Stefano Longhi, Marco Minutoli,
Christian Pilato, and others. It accepts standard C/C++ and compiler IRs from
both Clang/LLVM and GCC.

Key publications:
- Ferrandi et al., "Bambu: an Open-Source Research Framework for the High-Level
  Synthesis of Complex Applications" (DAC 2021, invited).
- Pilato & Ferrandi, "Bambu: A Modular Framework for the High Level Synthesis
  of Memory-Intensive Applications" (FPL 2013).

### 3.2  Architecture

Bambu's modular architecture accepts multiple compiler frontends (GCC, Clang)
and produces Verilog targeting both commercial (Vivado, Quartus) and open-source
(Yosys) synthesis backends.

**Scheduling.** Bambu implements a technology-aware scheduler that performs
aggressive operation chaining and code motion. The scheduler is aware of target
FPGA timing characteristics and can chain multiple operations into a single
clock cycle when timing permits.

**Binding.** Functional-unit binding with resource sharing using clique
partitioning on compatibility graphs.

**Optimizations.** Includes CSE, dead code elimination, loop transformations,
and memory optimization passes.

### 3.3  CHStone Results

Bambu publishes detailed CHStone results on their website
(panda.deib.polimi.it). Target: AMD/Xilinx Virtex-7 at 200 MHz.

Metrics used:
- **Latency**: clock cycles x achieved period post-implementation (nanoseconds)
- **Area**: Equivalent LUTs = BRAMs x 40 + DRAMs x 40 + DSPs x 40 +
  Registers x 0.5 + LUTs

Bambu provides Pareto plots (Latency vs. Area) for each benchmark, with
different `--experimental-setup` options steering the tool towards different
performance/area trade-offs. Comparisons with Vivado HLS are provided where
possible.

Published aggregate results:
- **CHStone suite**: Average 6% speedup over baseline with Bambu's optimization
  passes enabled.
- **Rosetta digit recognition**: 14% speedup with tensor optimization
  methodology integrated into Bambu (Ferrandi et al., 2020).

### 3.4  Comparison with Vivado HLS

The TU Delft survey (Nane et al., "A Survey and Evaluation of FPGA High-Level
Synthesis Tools," IEEE TCAD 2016) compared BAMBU, DWARV, and LEGUP against a
commercial tool (Vivado HLS). Key findings:
- Each tool excels on different benchmark subsets.
- Bambu's aggressive operation chaining gives it an edge on latency-critical
  kernels.
- Vivado HLS generally produces better-optimized designs for loop-heavy code
  due to superior pragma-driven pipelining.

### 3.5  Relevance to c2v

Bambu is the closest open-source competitor. Its technology-aware scheduling
(operation chaining guided by FPGA timing models) is something c2v should
match. However, Bambu's GCC/LLVM frontend dependency makes it heavyweight.
c2v's direct C-to-CDFG path avoids this overhead entirely. On the restricted-C
subset, c2v should be able to outperform Bambu by exploiting the stronger
static guarantees (no aliasing, no heap) that Bambu's general-purpose analysis
cannot assume.

---

## 4  CIRCT / Calyx (Cornell)

### 4.1  Calyx --- The Intermediate Language

Calyx is an intermediate language for compiling high-level programs into
hardware designs, developed by Rachit Nigam, Samuel Thomas, Zhijing Li, and
Adrian Sampson at Cornell University. Published at ASPLOS 2021: "A Compiler
Infrastructure for Accelerator Generators."

Calyx's key innovation is combining a **hardware-like structural language** with
a **software-like control language**:

- **Structural**: Wires, registers, memories, combinational groups --- standard
  RTL building blocks.
- **Control**: `seq`, `par`, `while`, `if` --- software control flow operators
  that orchestrate when structural groups are active.

This split representation enables optimizations that require both structural and
control information --- something neither pure-RTL nor pure-software IRs can
express naturally.

### 4.2  Results

From the ASPLOS 2021 evaluation:
- Systolic arrays generated through Calyx are **4.6x faster** and **1.11x
  larger** on average than equivalent HLS implementations.
- The speed advantage comes from Calyx's ability to exploit the regularity of
  systolic array control flow, generating tighter FSMs than HLS tools produce.

### 4.3  CIRCT Infrastructure

CIRCT (Circuit IR Compilers and Tools) is an LLVM/MLIR sub-project providing a
collection of hardware-oriented MLIR dialects. Calyx is one dialect within
CIRCT. Other relevant dialects include:

- **FIRRTL**: Chisel's intermediate representation.
- **HW/Comb/Seq**: Low-level hardware primitives.
- **Handshake**: Dataflow/dynamic scheduling representation.
- **ESI** (Elastic Silicon Interconnect): Communication protocols.

Recent work (2025) demonstrates end-to-end compilation from PyTorch models to
synthesisable SystemVerilog through MLIR, leveraging domain-specific dialects
to preserve high-level tensor structure and CIRCT infrastructure for hardware
lowering (arxiv 2512.06177).

### 4.4  Relevance to c2v

Calyx's control/structural split is conceptually similar to c2v's separation
of scheduling (control) and binding (structural). The 4.6x speedup over HLS
on systolic arrays shows what is possible when the compiler can exploit
structural regularity --- exactly the opportunity c2v has with restricted C.
However, Calyx is an IL, not a C compiler; it requires a frontend to lower C
to Calyx. c2v's direct approach avoids this translation overhead.

---

## 5  Dynamatic (EPFL)

### 5.1  Dynamic Scheduling Approach

Dynamatic is an academic, open-source HLS compiler from the Processor
Architecture Laboratory (LAP) at EPFL, led by Paolo Ienne. It produces
synchronous dynamically-scheduled circuits from C/C++ code.

Key publication: Lana Josipovic, Radhika Ghosal, and Paolo Ienne, "Dynamically
Scheduled High-Level Synthesis," FPGA 2018, pp. 127--136. **Best Paper Award
Nominee.**

In dynamic scheduling, components communicate locally using a **handshake
mechanism** and exchange data as soon as all conditions for a transaction are
satisfied. This contrasts with static scheduling (used by Vitis, LegUp, Bambu,
and c2v) where the schedule is fixed at compile time.

### 5.2  When Dynamic Beats Static

Dynamic scheduling excels when:
- **Irregular memory accesses**: Variable-latency memory operations
  (cache misses, DRAM access) cannot be statically predicted. Static tools
  must insert worst-case stalls.
- **Control-dominated code**: Many branches with data-dependent conditions.
  Static scheduling must conservatively assume the worst-case path.
- **Variable-latency operations**: Division, floating-point operations with
  data-dependent latency.

### 5.3  Area Overhead

The dynamic handshake mechanism adds significant area overhead:
- **Average area overhead**: 2.7x compared to static scheduling (Dynamatic).
- **DASS** (Cheng et al., FPGA 2020, TCAD 2021): A hybrid approach combining
  dynamic and static scheduling achieves 1.8x overhead, reducing Dynamatic's
  penalty.
- **Worst cases**: bubbleSort (many basic blocks, few instructions per block
  = high ratio of control to compute) and BNN (bit-level logic optimized
  more aggressively by static tools).
- **DSP overhead**: Dynamatic uses on average 1.6x more DSPs than static
  scheduling.

### 5.4  Dynamatic on MLIR (2025--2026)

Dynamatic has been rewritten on top of MLIR ("Dynamatic Reloaded"). Jiahui Xu,
Emmet Murphy, and Lana Josipovic presented "Is It a Good Idea to Build an HLS
Tool on Top of MLIR? Experience from Building the Dynamatic HLS Compiler" at
the Workshop on Languages, Tools, and Techniques for Accelerator Design (LATTE),
Pittsburgh, March 2026 (arxiv 2603.19856).

Related: Jiahui Xu and Lana Josipovic, "CRUSH: A Credit-Based Approach for
Functional Unit Sharing in Dynamically Scheduled HLS," ASPLOS 2025.

### 5.5  Relevance to c2v

Dynamic scheduling is **not relevant** to c2v's restricted-C subset. Our
programs have no heap (so no cache misses), no recursion (so no variable-depth
call stacks), no aliasing (so no memory hazards), and fixed arrays (so no
variable-latency memory). Static scheduling is strictly superior for this
domain. Dynamatic's 2.7x area overhead validates our choice of static
scheduling.

However, Dynamatic's MLIR rewrite experience is instructive: the LATTE 2026
paper discusses challenges of building HLS on MLIR, confirming that a
purpose-built IR (like c2v's internal CDFG) can avoid significant engineering
overhead.

---

## 6  ScaleHLS (UIUC / Ye et al.)

### 6.1  Multi-Level MLIR Optimization

ScaleHLS is a scalable HLS framework built on MLIR, developed by Hanchen Ye,
Cong Hao, and others. Published at HPCA 2022: "ScaleHLS: A New Scalable
High-Level Synthesis Framework on Multi-Level Intermediate Representation."

The key insight is performing HLS optimizations at **multiple representation
levels** within the MLIR infrastructure:
- **Graph level**: Dataflow optimizations across computation kernels.
- **Loop level**: Loop transformations (tiling, unrolling, pipelining).
- **Directive level**: Pragma-equivalent optimizations applied automatically.

### 6.2  Results

Evaluated on Xilinx XC7Z020 and VU9P FPGAs:
- Up to **768.1x improvement** over baseline unoptimized Vivado HLS on
  computation kernel benchmarks.
- Up to **3825.0x improvement** on neural network models.
- Optimization time: **37.3 to 60.8 seconds** for large, complicated designs
  with a single command.

These numbers compare against *unoptimized* baselines (no pragmas). The
comparison is against Vivado HLS without user pragmas, which is the fair
comparison point for an automatic tool.

### 6.3  Relevance to c2v

ScaleHLS's multi-level optimization approach validates the principle that
different optimizations should operate at different abstraction levels. c2v's
pass pipeline (CSE, reassociation, strength reduction, width narrowing) operates
at the CDFG level; adding graph-level and loop-level passes could yield similar
multiplicative improvements. The 37--61 second optimization time is a good
target for c2v's compile time on complex designs.

---

## 7  Jane Street Hardcaml

### 7.1  OCaml Hardware Library

Hardcaml is an OCaml library for designing and testing hardware, developed and
maintained by Jane Street. Published at FPGA 2024: "Hardcaml: An OCaml Hardware
Domain-Specific Language for Efficient and Robust Design."

Unlike HLS tools that compile C to hardware, Hardcaml is a **hardware
construction library**: designers write OCaml programs that *generate* hardware.
This gives:
- **Type safety**: OCaml's type system prevents wire-width mismatches,
  clock-domain crossing errors, and other bugs that plague Verilog.
- **Metaprogramming**: Higher-order functions, parametric polymorphism, and
  modules enable highly reusable hardware generators.
- **Simulation**: Native OCaml cycle-accurate simulator + event-driven
  simulator, plus backend support for Verilator and commercial simulators.

### 7.2  Production Use

Hardcaml is **industrially proven** at Jane Street for production FPGA designs.
Their team won **1st place in the FPGA track** of the 2022 ZPrize cryptography
competition for zk-SNARK acceleration on FPGAs.

### 7.3  Advent of Hardcaml --- Benchmarking Philosophy

The "Advent of Hardcaml" project (blog.janestreet.com, 2024) solves Advent of
Code puzzles entirely on FPGAs. The 2025 iteration became the "Advent of FPGA
Challenge" --- an open community challenge.

The benchmarking philosophy is revealing:
- **Competitive programming as hardware benchmark**: AoC puzzles require graph
  traversal, sorting, dynamic programming, and recursion --- algorithms that
  stress HLS tools in ways that traditional DSP/ML benchmarks do not.
- **Constrained resources**: Targeting a small FPGA (84K LUTs, 4 Mbit RAM)
  forces algorithmic optimizations that mirror real embedded constraints.
- **I/O realism**: Puzzles require realistic I/O (parsing text input, producing
  text output), testing the full system, not just the compute kernel.

### 7.4  Relevance to c2v

Hardcaml is not a direct competitor (it is a hardware construction library, not
an HLS compiler), but its benchmarking philosophy is instructive. The Advent of
Code approach tests:
1. **Algorithmic diversity** --- unlike CHStone/PolyBench which focus on
   DSP/linear algebra.
2. **Small kernel efficiency** --- where Vitis HLS control overhead dominates.
3. **Compile-time productivity** --- OCaml's type checker catches errors that
   would require simulation in Verilog.

c2v should adopt a similar philosophy: benchmark on diverse algorithmic kernels
where our restricted-C subset is expressive enough, and where Vitis's pragma
overhead and control bloat are most visible.

---

## 8  Benchmark Suites

### 8.1  CHStone (Hara et al., JIP 2008)

**12 benchmarks** for C-based HLS synthesisability testing, spanning four
application domains:

| Benchmark | Domain          | Description                              |
|-----------|-----------------|------------------------------------------|
| ADPCM     | Media           | Adaptive differential PCM encoder/decoder|
| AES       | Security        | Advanced Encryption Standard             |
| Blowfish  | Security        | Blowfish block cipher                    |
| DFADD     | Arithmetic      | Double-precision FP addition             |
| DFDIV     | Arithmetic      | Double-precision FP division             |
| DFMUL     | Arithmetic      | Double-precision FP multiplication       |
| DFSIN     | Arithmetic      | Double-precision sine function           |
| GSM       | Media           | GSM linear predictive coding             |
| JPEG      | Media           | JPEG image decompression                 |
| MIPS      | Microprocessor  | Simplified MIPS processor                |
| Motion    | Media           | MPEG-2 motion vector decoding            |
| SHA       | Security        | Secure Hash Algorithm                    |

**Published results on Virtex-7 @ 200 MHz (Bambu):**

The Bambu project publishes per-benchmark Pareto-optimal results with Equivalent
LUT and latency (ns) metrics. ADPCM is the most resource-diverse benchmark
(56--410K equivalent LUTs depending on optimisation level). Loop unrolling is
applied to ADPCM, and SHA for aggressive optimisation; pipelining yields up
to 80% latency reduction on ADPCM.

**c2v target**: All 12 CHStone benchmarks are within restricted C (no heap,
fixed arrays). c2v should match or beat Bambu/Vitis on every benchmark in this
suite. Priority targets: AES, SHA, ADPCM (security and media kernels where
control overhead matters most).

### 8.2  PolyBench (Pouchet, 2010)

**30 benchmarks** for polyhedral analysis, all with static control flow.
Representative kernels:

| Category          | Kernels                                           |
|-------------------|---------------------------------------------------|
| Linear algebra    | 2mm, 3mm, atax, bicg, gemm, gemver, gesummv, mvt  |
|                   | syrk, syr2k, trmm, cholesky, durbin, gramschmidt  |
|                   | lu, ludcmp, trisolv                               |
| Data mining       | correlation, covariance                           |
| Stencils          | adi, fdtd-2d, heat-3d, jacobi-1d, jacobi-2d       |
|                   | seidel-2d                                         |
| Solvers           | deriche, floyd-warshall, nussinov                 |

PolyBench is the **ideal benchmark suite for c2v** because:
- All programs have static control flow (no data-dependent branches).
- All arrays are fixed-size.
- No heap allocation or recursion.
- Kernels are compute-intensive with regular memory access patterns.
- Nine benchmarks are standard for HLS evaluation (gemm, 2mm, 3mm, atax,
  bicg, syrk, syr2k, mvt, gesummv).

Bambu publishes PolyBench results at panda.deib.polimi.it with Equivalent LUT
and latency metrics on Virtex-7 @ 200 MHz.

### 8.3  MachSuite (Reagen et al., IISWC 2014)

**19 benchmarks across 12 kernels** designed for accelerator design evaluation.
Developed at Harvard by Brandon Reagen, Bob Adolf, Yakun Sophia Shao, Gu-Yeon
Wei, and David Brooks.

Application domains:
- **Linear algebra / ML**: GeMM, KNN, Conv2D
- **Image processing**: Stencil3D
- **Speech recognition**: Viterbi
- **Signal processing**: FFT, FIR
- **Security**: AES
- **Graph algorithms**: BFS, Sort (merge, radix)
- **Dynamic programming**: NW (Needleman-Wunsch)

Multiple implementation variants per kernel capture distinct algorithmic choices
(e.g., blocked vs. non-blocked matrix multiply). All benchmarks are HLS-
synthesisable.

**c2v coverage**: Most MachSuite kernels are within restricted C. Exceptions:
BFS (graph traversal with pointer-like indirection) may require adaptation.
Priority targets: GeMM, FFT, FIR, AES, Stencil3D.

### 8.4  Rosetta (Zhou et al., FPGA 2018)

**6 application-level benchmarks** from ML and image/video processing. Developed
at Cornell by Yuan Zhou and others. Each application contains multiple compute
kernels with diverse parallelism sources.

Applications: optical flow, spam filter, digit recognition, face detection,
3D rendering, and BNN (binary neural network).

Rosetta differs from CHStone/PolyBench/MachSuite in being **application-level**
rather than kernel-level: each benchmark is a full pipeline with multiple
kernels, realistic data sizes, and application-level optimization constraints.

**c2v coverage**: Individual kernels within Rosetta applications are within
restricted C, but the full application-level integration may require features
beyond our current scope. Rosetta is a **stretch goal** for c2v.

### 8.5  Modern Benchmark Frameworks (2024--2026)

Recent benchmark developments:

- **HLSFactory** (MLSys/MLCAD 2024): Framework for generating HLS datasets
  for ML-guided optimisation. Provides standardised interfaces for comparing
  HLS tool outputs.

- **ForgeHLS / ForgeBench** (2025): Large-scale open-source HLS datasets with
  auto-generation framework. Contains benchmarks derived from CHStone, PolyBench,
  MachSuite, and Rosetta through design-space exploration.

- **HLS-Eval** (2025): Benchmark and framework for evaluating LLMs on HLS
  design tasks. Uses CHStone, PolyBench, MachSuite, Rosetta, and custom
  benchmarks.

- **Bench4HLS** (2025): End-to-end evaluation of LLMs for HLS code generation,
  with standardised metrics.

### 8.6  Recommended c2v Benchmark Strategy

**Tier 1 (must beat Vitis):**
- CHStone full suite (12 benchmarks) --- well-understood, published baselines
- PolyBench compute kernels (9 standard HLS benchmarks) --- ideal for
  restricted C

**Tier 2 (should beat Vitis):**
- MachSuite applicable kernels (GeMM, FFT, FIR, AES, Stencil3D, NW)
- Additional PolyBench kernels (stencils, solvers)

**Tier 3 (stretch goals):**
- Rosetta individual kernels
- Advent-of-Code-style algorithmic benchmarks (following Hardcaml philosophy)

**Metrics to report:**
- Clock cycles (latency)
- Post-route LUTs, FFs, BRAMs, DSPs
- Equivalent LUTs (Bambu formula: BRAMs x 40 + DSPs x 40 + Regs x 0.5 + LUTs)
- Fmax (achieved clock frequency post-route)
- Compile time (c2v time + Vivado synthesis/implementation time)
- Lines of input C (to normalise complexity)

---

## 9  DSP48 Inference Comparison Across Tools

### 9.1  DSP48E2 Architecture Recap

The DSP48E2 (UltraScale+) is a complex primitive offering 40+ dynamically
controlled operations:

```
 A[29:0] ──┐
            ├─ Pre-adder (A+D or A-D, 27-bit) ──┐
 D[26:0] ──┘                                     ├─ Multiplier (27x18) ──┐
 B[17:0] ────────────────────────────────────────┘                       │
                                                                          ├─ ALU (48-bit)
 C[47:0] ────────────────────────────────────────────────────────────────┘
                                                                          │
 P[47:0] ←── Pattern detect ←── Output register ←───────────────────────┘
                                        ↕
                              PCIN/PCOUT cascade (column)
```

Key capabilities:
- **27x18 signed multiplier** (vs. 25x18 in DSP48E1)
- **27-bit pre-adder** (enables symmetric FIR: (A+D)*B in one DSP)
- **48-bit ALU** with SIMD support (two 24-bit or four 12-bit operations)
- **Pattern detector** for convergent rounding and overflow detection
- **Wide XOR** (96-bit) for CRC and security applications
- **Cascade**: Up to 120 DSPs in a column via PCIN/PCOUT ports
- **Fmax**: Up to 891 MHz

### 9.2  Inference Quality by Tool

**Vitis HLS:**
- Infers basic multiply, MAC, and multiply-add patterns.
- Pre-adder inference requires specific coding patterns: `(a+d)*b` must be
  written explicitly; the compiler will not discover this from `a*b + d*b`.
- Cascaded DSP chains are inferred for accumulation loops.
- **Weakness**: Pre-adder utilisation is low in practice. Symmetric FIR filters
  require manual coding of the pre-add pattern. The tool does not perform the
  algebraic transformation `a*b + d*b --> (a+d)*b` to enable pre-adder mapping.
- SILVIA (Ottati et al., ACM TRETS 2024) addresses this gap: an open-source
  LLVM pass for Vitis HLS that packs multiple operations into single DSPs,
  reducing DSP utilisation for additions by **70%** and for multiplications/MACs
  by **50%** on average.

**Bambu:**
- Technology-aware scheduling chains operations into DSP patterns.
- DSP inference is more conservative than Vitis for complex patterns.
- Does not specifically target pre-adder utilisation.

**LegUp/SmartHLS:**
- DSP inference through Vivado/Quartus backend during logic synthesis.
- The HLS tool itself does not perform DSP-aware scheduling.

**Hand-written RTL:**
- Vivado synthesis infers DSP48 from behavioural Verilog patterns:
  `P <= (A + D) * B` maps to pre-adder + multiplier.
- Cascade chains require explicit coding or careful accumulation patterns.
- 30% fewer LUTs than HLS for equivalent functionality in multiply-heavy
  designs (arxiv 2509.08067).

### 9.3  c2v DSP Inference Strategy

c2v has a unique advantage: the restricted-C subset means we can perform
algebraic transformations that general-purpose compilers cannot safely apply
(due to aliasing, side effects, or precision concerns). Specific opportunities:

1. **Pre-adder exploitation**: In the reassociation pass, detect patterns like
   `a*b + d*b` and rewrite to `(a+d)*b`, enabling pre-adder mapping. This is
   the transformation that Vitis HLS misses.

2. **Cascaded MAC chains**: For accumulation loops (`for(i) sum += a[i]*b[i]`),
   generate a chain of DSPs connected via PCIN/PCOUT cascade ports. Each DSP
   computes one multiply-accumulate. The cascade avoids routing through fabric.

3. **Narrow-width packing**: The DSP48E2's SIMD mode allows two 24-bit or four
   12-bit additions in a single DSP. The width-narrowing pass should identify
   narrow operands and pack them.

4. **Symmetric filter detection**: For FIR-like patterns with symmetric
   coefficients, detect and exploit the pre-adder to halve DSP count.

5. **Constant multiplication**: For multiply-by-constant, use shifts and adds
   when the constant is sparse, or use the DSP when it is dense. The strength-
   reduction pass should make this decision based on the target's DSP/LUT
   ratio.

---

## 10  HLS Frontier 2024--2026

### 10.1  AutoDSE and Design Space Exploration

**AutoDSE** (UCLA VAST, ACM TODAES Best Paper 2022): Automated design-space
exploration using bottleneck-guided coordinate optimisation.
- Achieves **19.9x geomean speedup** over single CPU core on MachSuite and
  Rodinia benchmarks.
- Reduces optimisation pragmas by **26.38x** versus Xilinx Vitis library
  kernels while achieving similar performance.
- Each design point requires 3--5 hours of Vitis compilation --- a week per
  benchmark.

**AutoScaleDSE** (Sohrabizadeh et al., ACM TRETS 2023): Scales AutoDSE to
larger designs using hierarchical decomposition.

**AutoHLS** (MERL, 2024): Uses ML to predict design quality and guide
exploration. Formulates pragma insertion as multi-objective optimisation over
unroll factor, pipeline depth, array partition, and inline decisions.

### 10.2  ML-Guided Optimisation

**LLM-DSE** (2025): Multi-agent LLM framework for HLS directive optimisation.
Four agents (Router, Specialists, Arbitrator, Critic) collaborate to explore
the design space. Achieves **2.55x performance gain** over state-of-the-art on
the HLSyn dataset.

**MPM-LLM4DSE** (2025): Integrates multimodal prediction (behavioural
descriptions + CDFGs) with LLM-guided exploration. **39.90% average performance
gain** over previous DSE methods.

**iDSE** (2025): Outperforms heuristic-based DSE by **5.1x--16.6x** in
proximity to the reference Pareto front, matching NSGA-II with only 4.6% of
explored designs.

**TimelyHLS** (2025): LLM-based timing-aware and architecture-specific FPGA
HLS optimisation (arxiv 2507.17962).

### 10.3  Equality Saturation and E-Graph Rewriting

**Diospyros** (Nigam et al., ASPLOS 2021): Vectorisation of DSP code via
equality saturation. Lifts scalar code to a vector DSL using symbolic
evaluation, then exhaustively searches the space of equivalent programs using
e-graphs. Foundation for hardware-oriented e-graph work.

**E-Syn** (Chen et al., DAC 2024): E-graph rewriting with technology-aware cost
functions for logic synthesis. Results: **15.29% delay improvement** for delay-
oriented optimisation vs. AIG-based ABC, **6.42% area saving** for area-oriented
synthesis.

**E-morphic** (2025): Scalable equality saturation for structural exploration in
logic synthesis (arxiv 2504.11574).

**eqsat MLIR dialect** (EGRAPHS 2025, PLDI workshop, Seoul): Represents e-graphs
natively in MLIR, enabling equality saturation as a standard compiler pass.
Avoids translation overhead between e-graph libraries and MLIR. Enables
orchestration of existing MLIR passes (like CSE) while maintaining e-graph
state throughout the compilation flow (arxiv 2505.09363).

**DialEgg** (2025): Dialect-agnostic MLIR optimiser using equality saturation
with Egglog. Applies e-graph rewriting to arbitrary MLIR dialects.

### 10.4  Relevance to c2v

E-graph rewriting is **directly applicable** to c2v's optimisation passes.
Current passes (CSE, reassociation, strength reduction, width narrowing) apply
transformations greedily and sequentially. An e-graph approach would:

1. Represent all equivalent forms of an expression simultaneously.
2. Apply all rewrite rules (reassociation, distribution, constant folding,
   DSP pattern matching) in parallel.
3. Extract the optimal implementation using a cost function that accounts for
   target resources (LUTs, DSPs, BRAMs, routing delay).

This is a medium-term goal. The immediate priority is proving c2v beats Vitis
on existing benchmarks with the current greedy passes; e-graph rewriting is
the next-generation optimisation layer.

### 10.5  Other Notable 2024--2026 Work

**SILVIA** (Ottati et al., ACM TRETS 2024): Superword-level parallelism
exploitation via HLS-specific LLVM passes for Vitis HLS. Reduces DSP
utilisation for additions by 70%, multiplications/MACs by 50%. Open-source
at github.com/brigio345/SILVIA.

**HACE** (FPGA 2026): HLS-tool-agnostic CDFG extraction from RTL designs.
Enables post-hoc analysis of any HLS tool's output, useful for competitive
benchmarking.

**Wavefront Threading** (2024): Enables effective HLS by discovering parallelism
through wavefront analysis (arxiv 2405.19514).

**CRUSH** (Josipovic, ASPLOS 2025): Credit-based functional unit sharing in
dynamically scheduled HLS. Reduces area overhead of dynamic scheduling.

**Subgraph Extraction-based Scheduling** (2024): Feedback-guided iterative
scheduling for HLS using subgraph extraction (arxiv 2401.12343).

**Differentiable Scheduling** (2026): CPU-GPU hybrid combinatorial scheduling
using differentiable initialisation (arxiv 2603.28943). Explores using gradient-
based methods to initialise combinatorial scheduling solvers.

---

## 11  Competitive Positioning Summary

### 11.1  Tool Comparison Matrix

| Feature              | Vitis HLS | LegUp/SmartHLS | Bambu    | Calyx   | Dynamatic | ScaleHLS | c2v (target) |
|----------------------|-----------|----------------|----------|---------|-----------|----------|--------------|
| Frontend             | Clang/LLVM| Clang/LLVM     | GCC+Clang| DSL/IL  | Clang     | MLIR     | Direct C AST |
| Scheduling           | SDC+prop. | SDC            | Tech-aware| FSM gen| Dynamic   | MLIR     | SDC (LP)     |
| Binding              | Proprietary| Graph-based   | Clique   | Manual  | Handshake | MLIR     | Graph color  |
| Open source          | Frontend  | No (SmartHLS)  | Yes      | Yes     | Yes       | Yes      | Yes          |
| Compile time (typ.)  | Minutes-hrs| Minutes       | Minutes  | Seconds | Minutes   | 37-61s   | Seconds      |
| Restricted-C focus   | No        | No             | No       | No      | No        | No       | Yes          |
| No LLVM dependency   | No        | No             | No       | N/A     | No        | No       | Yes          |
| DSP48 pre-adder      | Limited   | Via backend    | Limited  | Manual  | No        | No       | Planned      |
| Target FPGAs         | Xilinx    | Microchip      | Multi    | Multi   | Xilinx    | Xilinx   | Xilinx+Intel |

### 11.2  Where c2v Wins

1. **No LLVM overhead**: Direct C-to-CDFG eliminates multi-second frontend
   compilation, 100+ MB memory overhead, and LLVM's general-purpose IR bloat.

2. **No pragma needed**: The restricted-C subset is simple enough that the
   compiler can make optimal decisions without user hints. Vitis HLS requires
   expert pragma tuning to approach optimal performance.

3. **No control overhead**: No ap_ctrl handshake, no interface adapters, no
   AXI wrappers. Pure datapath + minimal FSM. This alone should save 20--30%
   LUTs on small kernels.

4. **Algebraic optimisation**: CSE, reassociation, strength reduction, and
   width narrowing operate on a representation with guaranteed no-alias,
   no-side-effect semantics. Transformations that are unsafe in general C
   (like `a*b + d*b --> (a+d)*b` for pre-adder mapping) are provably safe
   in restricted C.

5. **Compile time**: Seconds vs. minutes-to-hours. This enables rapid design
   iteration and automated design space exploration without the 3--5 hour
   per-point cost of Vitis-based DSE.

6. **Deterministic scheduling**: SDC with LP solver produces provably optimal
   latency for the given resource constraints. No heuristic variance.

### 11.3  Where c2v Must Catch Up

1. **Loop pipelining**: Modulo SDC scheduling (Canis et al., FPL 2014) is
   essential for competitive performance on loop-heavy benchmarks. This is
   already planned.

2. **Technology-aware scheduling**: Bambu's operation chaining (multiple ops
   per cycle when timing permits) is critical for high Fmax. c2v needs target-
   specific timing models.

3. **DSP inference**: Must match or beat Vitis on multiply/MAC mapping and
   exceed it on pre-adder utilisation.

4. **Memory partitioning**: Automatic array partitioning (cyclic, block,
   complete) based on access pattern analysis.

5. **Benchmark coverage**: Need to run all Tier 1 and Tier 2 benchmarks
   through c2v and compare against published Bambu/Vitis numbers.

---

## 12  Benchmark Execution Plan

### 12.1  Phase 1 --- Establish Baselines (Immediate)

1. Compile all 12 CHStone benchmarks with Vitis HLS 2024.2 targeting
   Virtex-7 (xc7vx485t) and UltraScale+ (xcvu9p) at 200 MHz.
   - Record: cycles, LUTs, FFs, DSPs, BRAMs, Fmax, compile time.
   - Use default pragmas (no optimisation) and optimised pragmas.

2. Download Bambu CHStone results from panda.deib.polimi.it for comparison.

3. Compile 9 standard PolyBench kernels (gemm, 2mm, 3mm, atax, bicg, syrk,
   syr2k, mvt, gesummv) with Vitis HLS under the same conditions.

### 12.2  Phase 2 --- c2v Comparison (Near-term)

1. Implement CHStone benchmarks in restricted C (adapting where necessary to
   remove heap/recursion if present).

2. Compile with c2v targeting the same Virtex-7 and UltraScale+ devices.

3. Run Vivado synthesis + implementation on c2v's generated Verilog.

4. Compare all metrics. Publish results as Pareto plots (latency vs. area)
   following Bambu's methodology.

### 12.3  Phase 3 --- Optimisation-Driven (Medium-term)

1. For benchmarks where c2v loses, identify the gap:
   - Scheduling gap: c2v schedules more cycles than Vitis.
   - Binding gap: c2v uses more resources than Vitis.
   - DSP gap: c2v maps fewer operations to DSPs.
   - Fmax gap: c2v's generated RTL has longer critical paths.

2. Implement targeted optimisations (loop pipelining, technology-aware
   scheduling, DSP inference) and re-benchmark.

3. Target: beat Vitis HLS on **all** CHStone benchmarks for at least one
   metric (latency or area) while being competitive on the other.

### 12.4  Phase 4 --- Publication (Long-term)

1. Extend to MachSuite and PolyBench full suite.
2. Compare against Bambu, Calyx, and ScaleHLS where published numbers exist.
3. Include compile-time comparison (c2v seconds vs. Vitis minutes/hours).
4. Target venue: FPGA, FCCM, or DAC.

---

## References

### Core Tools

- Cong, Zhang. "An Efficient and Versatile Scheduling Algorithm Based on SDC
  Formulation." DAC 2006, pp. 433--438.
- Canis, Choi, Aldham, Zhang, Kammoona, Anderson, Brown, Czajkowski. "LegUp:
  High-Level Synthesis for FPGA-Based Processor/Accelerator Systems." FPGA 2011.
- Canis, Brown, Anderson. "Modulo SDC Scheduling with Recurrence Minimization
  in High-Level Synthesis." FPL 2014.
- Ferrandi et al. "Bambu: an Open-Source Research Framework for the High-Level
  Synthesis of Complex Applications." DAC 2021 (invited).
- Pilato, Ferrandi. "Bambu: A Modular Framework for the High Level Synthesis
  of Memory-Intensive Applications." FPL 2013.
- Nigam, Thomas, Li, Sampson. "A Compiler Infrastructure for Accelerator
  Generators." ASPLOS 2021.
- Josipovic, Ghosal, Ienne. "Dynamically Scheduled High-Level Synthesis."
  FPGA 2018, pp. 127--136.
- Ye, Hao et al. "ScaleHLS: A New Scalable High-Level Synthesis Framework on
  Multi-Level Intermediate Representation." HPCA 2022.

### Benchmark Suites

- Hara et al. "CHStone: A Benchmark Program Suite for Practical C-Based
  High-Level Synthesis." JIP 2008.
- Pouchet. "PolyBench: The Polyhedral Benchmark Suite." 2010.
- Reagen, Adolf, Shao, Wei, Brooks. "MachSuite: Benchmarks for Accelerator
  Design and Customized Architectures." IISWC 2014.
- Zhou et al. "Rosetta: A Realistic High-Level Synthesis Benchmark Suite for
  Software Programmable FPGAs." FPGA 2018.

### Surveys and Comparisons

- Nane et al. "A Survey and Evaluation of FPGA High-Level Synthesis Tools."
  IEEE TCAD 2016.

### DSP and Optimisation

- SILVIA: Ottati et al. "Automated Superword-Level Parallelism Exploitation
  via HLS-Specific LLVM Passes." ACM TRETS 2024.
- E-Syn: Chen et al. "E-Graph Rewriting with Technology-Aware Cost Functions
  for Logic Synthesis." DAC 2024.
- AutoDSE: Sohrabizadeh et al. "AutoDSE: Enabling Software Programmers to
  Design Efficient FPGA Accelerators." ACM TODAES Best Paper 2022.

### Frontier 2024--2026

- LLM-DSE: "Searching Accelerator Parameters with LLM Agents." 2025.
  (arxiv 2505.12188)
- MPM-LLM4DSE: "Reaching the Pareto Frontier in HLS with Multimodal Learning
  and LLM-Driven Exploration." 2025. (arxiv 2601.04801)
- iDSE: "Navigating Design Space Exploration in HLS Using LLMs." 2025.
  (arxiv 2505.22086)
- Xu, Murphy, Josipovic. "Is It a Good Idea to Build an HLS Tool on Top of
  MLIR?" LATTE 2026. (arxiv 2603.19856)
- Xu, Josipovic. "CRUSH: Credit-Based Functional Unit Sharing in Dynamically
  Scheduled HLS." ASPLOS 2025.
- eqsat: "An Equality Saturation Dialect for Non-destructive Rewriting."
  EGRAPHS/PLDI 2025. (arxiv 2505.09363)
- E-morphic: "Scalable Equality Saturation for Structural Exploration in
  Logic Synthesis." 2025. (arxiv 2504.11574)
- HACE: "HLS-Tool-Agnostic CDFG Extraction from RTL Designs." FPGA 2026.
- Differentiable Scheduling: "Differentiable Initialization-Accelerated
  CPU-GPU Hybrid Combinatorial Scheduling." 2026. (arxiv 2603.28943)
- Hardcaml: "An OCaml Hardware Domain-Specific Language for Efficient and
  Robust Design." FPGA 2024.
- Advent of FPGA: blog.janestreet.com/advent-of-fpga-challenge-2025/
