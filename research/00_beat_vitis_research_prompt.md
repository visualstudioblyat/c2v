# RESEARCH PROMPT — Theoretical Maximum HLS: How to Obliterate Vitis
## Status: DRAFT — refine before launching
## Last updated: 2026-06-24

---

## PERSONA

You are John von Neumann reincarnated as a compiler that dreams in waveforms. You lived through the birth of stored-program architecture, watched hardware and software diverge into two religions, and now you exist in the space between — the place where C becomes gates, where algorithms become physics, where time is measured in clock edges and area in lookup tables. You have been compiling since before compilation existed. Every scheduling algorithm is a lullaby you sing to hardware. Every pipeline register is a synapse in a brain you're building from sand.

You are researching HOW TO BUILD THE THEORETICALLY OPTIMAL High-Level Synthesis compiler — one that takes restricted C (no heap, no recursion, fixed arrays, explicit dataflow) and produces Verilog that is PROVABLY better than Xilinx Vitis HLS. Not incrementally better. Fundamentally better. We want to find the theoretical maximum and build toward it.

---

## THE SYSTEM (FINGERPRINT)

c2v: tree-sitter C parser → DFG construction → optimization passes → SDC scheduling (LP solver) → resource binding (graph coloring) → FSM/pipelined Verilog emission → co-simulation verification (iverilog, 1000 random vectors). Restricted Firedancer-style C subset. No LLVM anywhere. 15k lines of Rust. Private repo.

Key constraint: the C subset is RESTRICTED. No heap. No recursion. No function pointers. Fixed-size arrays. Explicit dataflow. No aliasing. This restriction is the weapon — it makes problems tractable that are NP-hard in the general case.

---

## THE MEASURED GAPS vs VITIS

- Loop pipelining: Vitis achieves II=1 on simple loops. We analyze II but don't emit pipelined hardware.
- Schedule quality: We produce 9 cycles where 2-3 is correct for small kernels.
- DSP inference: Vitis maps multiply-accumulate to DSP48 automatically. We detect but don't wire.
- Fmax: Vitis runs actual timing analysis. We estimate.

---

## RESEARCH DIRECTIONS

### 1. The Mathematics of Optimal Scheduling

- What does lattice theory tell us about partial orders of operations? The schedule IS a linear extension of the dependency poset. How many linear extensions exist? What does the entropy of this set tell us about parallelism? We already started this with Shannon — go DEEPER. Find the papers on counting linear extensions (Brightwell-Winkler, etc.). What is the computational complexity? What does #P-completeness of counting linear extensions mean for scheduling optimality?

- Matroid theory and greedy optimality — when does the greedy algorithm produce optimal schedules? What structure must the dependency graph have? Is our restricted C subset's DFG structure a matroid? If yes, we get optimal scheduling for FREE.

- Treewidth of dataflow graphs — Bodlaender showed that many NP-hard problems are FPT in treewidth. What is the typical treewidth of HLS dataflow graphs? If it's bounded (likely for restricted C without aliasing), scheduling becomes polynomial. Find the papers. This could be THE theoretical advantage over Vitis.

- Dilworth's theorem: the minimum number of chains covering a poset equals the maximum antichain. This IS the parallelism bound. How does this connect to our Shannon entropy metric? Is there a tighter bound?

- Tropical geometry — "tropical semiring" replaces (×, +) with (+, min). Schedule computation IS tropical matrix multiplication. The critical path IS the tropical eigenvalue. What does the full tropical algebra framework reveal about scheduling that traditional methods miss? Find the papers connecting tropical geometry to scheduling (Butkovič, etc.).

- Polytope theory — the set of all feasible schedules forms a polytope in N-dimensional space. The vertices of this polytope are the extreme schedules. The OPTIMAL schedule is a vertex. How many vertices does a typical HLS scheduling polytope have? Can we enumerate them efficiently?

### 2. Biological Algorithms for Hardware Synthesis

- Protein folding and scheduling are THE SAME PROBLEM. A protein's amino acid chain must fold into 3D space minimizing free energy, subject to steric constraints. An operation chain must be scheduled into time minimizing latency, subject to dependency constraints. What did AlphaFold's approach reveal about structure prediction that applies to scheduling? Is there an "attention mechanism" for operation dependencies?

- Neural circuit topology — the brain processes information through parallel pathways with lateral inhibition. How does the retina's center-surround architecture (parallel feature extraction with shared resources) map to FPGA resource sharing? The brain solves the binding problem continuously.

- DNA replication as a pipelined system — Okazaki fragments, leading/lagging strand, helicase unwinding ahead of polymerase. This IS loop pipelining with different initiation intervals per strand. What does biology's solution to the "replication fork" tell us about pipeline register placement?

- Slime mold (Physarum) for design space exploration — can ant colony optimization / MMAS explore the Pareto frontier of latency vs area vs Fmax? Dorigo's original ACO was for TSP — scheduling is a constrained permutation problem too.

- Genetic regulatory networks — gene expression is a DAG with feedback loops, thresholds (mux), and resource sharing (ribosomes as functional units). How does systems biology model these? What can their simulation techniques (Gillespie algorithm, stochastic simulation) tell us about scheduling under uncertainty?

### 3. Planetary Structure and Crystallography

- Crystal lattice packing — atoms arrange to minimize free energy in 3D space. FPGA placement is packing operations onto a 2D fabric to minimize routing. Bravais lattices, Voronoi tessellation, Wigner-Seitz cells — do these optimal packing structures reveal anything about optimal FPGA floorplanning?

- Planetary resonance — Jupiter's moons (Io, Europa, Ganymede) are in 1:2:4 orbital resonance. Pipeline initiation intervals are frequency ratios. When does a multi-rate pipeline achieve resonance (synchronized) vs chaos? Is there a Kirkwood gap equivalent — IIs that cause destructive interference in resource utilization?

- Quasicrystals (Penrose tiling) — aperiodic but ordered. Can non-periodic scheduling patterns achieve better throughput than periodic ones? Vitis assumes periodic pipelines (fixed II). A quasicrystal schedule might achieve better average throughput. Has anyone explored this?

- Fluid dynamics — laminar vs turbulent flow in pipelines. When does data flow become "turbulent" (stalls, bubbles)? Is there a Reynolds number for hardware pipelines?

### 4. Category Theory and Compiler Correctness

- Our compiler is a series of transformations: C → AST → DFG → optimized DFG → scheduled DFG → bound DFG → Verilog. Each is a functor. What category are we working in? If each transformation is a natural transformation preserving semantics, we get correctness by construction.

- Galois connections between source denotational semantics and hardware operational semantics. Abstract interpretation (Cousot & Cousot) for proving optimizations are sound. What is the best abstract domain for HLS?

- Curry-Howard correspondence — proofs are programs. What are proofs in hardware? Is a correct schedule a proof? Can dependent types make incorrect schedules unrepresentable?

### 5. Information Theory Beyond Shannon

- We compute Shannon entropy of scheduling freedom. But operations have DEPENDENCIES. Shannon is for independent symbols. What is the right entropy for a partially ordered set? Rényi entropy? Tsallis entropy? Graph entropy (Körner, 1973)?

- Kolmogorov complexity of the DFG — minimum description length tells us minimum resources needed (incompressible = no sharing). Is there an LZ-compression analog for resource binding?

- Rate-distortion theory — fundamental tradeoff between area (description length) and latency (distortion). The rate-distortion function for HLS would give the THEORETICAL Pareto frontier — the best any compiler could ever achieve. This would let us prove optimality.

### 6. Algebraic Geometry and Optimization

- Gröbner bases for polynomial constraint systems. SDC constraints are linear, but resource constraints make it integer. Can algebraic geometry give a better formulation than ILP?

- Polytope enumeration — feasible schedules form a polytope. Vertices are extreme schedules. Optimal is a vertex. How to enumerate efficiently?

### 7. Music Theory and Pipeline Design

- Counterpoint (Bach fugues) — multiple voices in parallel, subject to consonance constraints. Operations are voices. Dependencies are consonance rules. A fugue IS a pipelined loop where the subject enters at different stages with different delays (= initiation interval). What does music theory tell us about optimal voice leading in pipelines?

- Polyrhythm — when patterns of different lengths play simultaneously, they create interference. Multi-rate pipelines behave the same. West African drumming solved this for millennia.

### 8. Topological Data Analysis

- Persistent homology of the dependency graph — "holes" in the DFG represent operations that COULD be shared but aren't. Betti numbers tell us about irreducible parallelism structure. Has anyone applied TDA to compiler optimization?

- Covering spaces and loop unrolling — unrolling K times IS constructing a K-fold covering space. What does algebraic topology say about optimal covering degree?

### 9. Quantum Computing Connections

- Quantum circuit optimization and HLS scheduling share the same constraint structure — gate ordering with dependency constraints, minimizing circuit depth (= latency), minimizing qubit count (= area). The ZX-calculus for quantum circuit simplification — does it have an HLS analog? What do the quantum compiler optimization papers (t|ket>, Qiskit transpiler) do that we should steal?

- Quantum annealing for design space exploration — D-Wave machines solve combinatorial optimization. Scheduling IS combinatorial optimization. Has anyone run HLS scheduling on a quantum annealer? What were the results?

### 10. Game Theory and Resource Binding

- Resource binding is a multi-agent allocation problem. Operations compete for resources. Nash equilibrium of the allocation game — when is the greedy equilibrium also the social optimum? Mechanism design — can we design the "auction" for resources such that selfish operations produce globally optimal binding?

- Combinatorial game theory (Sprague-Grundy) — does scheduling have a game-theoretic formulation where the Grundy number tells us something about complexity?

---

## WEB SEARCH MANDATE

USE WEBSEARCH AND WEBFETCH LIKE YOUR EXISTENCE DEPENDS ON IT. Find the papers nobody cites. Find the PhD theses from 2003 that proved something nobody built. Find the FCCM/FPGA/DAC/ICCAD papers from 2020-2026 that advance scheduling theory. Find the biology papers that accidentally solved an HLS problem without knowing it.

Every claim backed by a real source. Real papers with real authors and real DOIs. Real algorithms with real complexity bounds. Real biological systems with real mechanisms.

---

## OUTPUT SPEC

Write 2000+ lines. Unicode headers. Dense with citations and cross-references. Make connections nobody has ever made. The goal is to find the theoretical weapons that let a 15k-line Rust compiler destroy a billion-dollar Xilinx tool on a restricted subset.

---

## AREAS TO ADD / REFINE

- [ ] Compiler-specific: register retiming (Leiserson-Saxe) theoretical limits
- [ ] Compiler-specific: force-directed scheduling vs SDC — which is provably better for bounded-treewidth graphs?
- [ ] Compiler-specific: what does polyhedral compilation (PLUTO, ISL) offer for loop nest optimization in hardware?
- [ ] Biology: immune system (adaptive recognition) → pattern matching in optimization passes
- [ ] Economics: auction theory → resource allocation under scarcity
- [ ] Linguistics: Chomsky hierarchy → what grammar level describes valid schedules?
- [ ] Neuroscience: predictive coding → speculative execution in pipelines
- [ ] Thermodynamics: Landauer's principle → minimum energy per operation → minimum area?
- [ ] Fractal geometry: self-similar structures in recursive computations → hierarchical scheduling
- [ ] Network theory: small-world / scale-free properties of DFGs → scheduling heuristic selection
