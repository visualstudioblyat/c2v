# Biological Algorithms for Hardware Synthesis

## Context and Motivation

c2v compiles restricted C (no heap, no recursion, fixed-size arrays, no pointer
aliasing) to synthesisable Verilog. We use SDC scheduling with an LP solver,
graph coloring for binding, tree balancing for reduction chains, and
entropy-directed heuristics. The strategic goal is to **beat Xilinx Vitis HLS**
on quality of results for the restricted-C subset we support.

Biology has spent 3.8 billion years optimising information-processing systems
under constraints that mirror those in HLS: limited energy (area), finite time
(latency), noisy channels (routing congestion), and combinatorial explosion
(design space). This document surveys seven biological domains whose algorithms
can be mined for ideas that directly improve c2v's compilation pipeline.

Every section maps the biological system onto a concrete HLS sub-problem, cites
real papers with authors and years, and identifies what c2v should steal.

---

## 1  Protein Folding and Scheduling

### 1.1  The Structural Parallel

Both protein folding and HLS scheduling minimise an objective function (free
energy / latency) subject to structural constraints (bond angles / data
dependencies) across a combinatorial space whose size grows exponentially with
input size.

**Levinthal's Paradox.** A 100-residue protein has roughly 3^200 possible
backbone conformations. Exhaustive search would take longer than the age of the
universe, yet real proteins fold in milliseconds (Levinthal, 1969). The
resolution: the energy landscape is **funnel-shaped** --- there exists a
gradient that guides the search toward the native state without exploring the
full space (Dill & Chan, 1997; Onuchic et al., 1997).

The same paradox applies to scheduling. A DFG with *n* operations and *L*
time-steps has O(L^n) candidate schedules. SDC scheduling escapes this by
exploiting the **total unimodularity** of the constraint matrix (Cong & Zhang,
2006), solving in polynomial time when resource constraints are approximated.
But when resources are tight, the LP relaxation is insufficient and we fall back
to heuristics. Biology's funnel-shaped landscape suggests a better approach:
engineer the objective function so that random perturbations almost always
improve the schedule, creating a gradient toward optimality.

**Protein folding is NP-hard.** Berger & Leighton (1998) proved that protein
folding in the HP lattice model is NP-hard. Crescenzi et al. (1998) extended
this to show that even approximate folding is hard. The structural analogy is
tight: HLS scheduling under resource constraints is also NP-hard (Garey &
Johnson, 1979), and both problems share the property that good solutions cluster
in a small region of the search space reachable via local moves.

### 1.2  AlphaFold's Evoformer and Attention over Dependencies

AlphaFold2 (Jumper et al., 2021, Nature) predicts protein structure using a
deep transformer architecture called the **Evoformer**. The Evoformer maintains
two coupled representations:

1. **MSA representation** --- a matrix of shape (sequences x residues) capturing
   evolutionary co-variation. Processed with row-wise gated self-attention (with
   pair bias) and column-wise gated self-attention.

2. **Pair representation** --- a matrix of shape (residues x residues) capturing
   pairwise geometric relationships. Processed with triangle multiplicative
   updates and triangle self-attention.

The key architectural insight: **information flows bidirectionally** between the
MSA and pair tracks through outer product mean operations. The pair
representation enforces geometric consistency via triangle updates --- if
residue A is close to B and B is close to C, then A and C must satisfy a
triangle inequality.

**HLS analogy.** In scheduling, we have:
- An **operation representation** (analogous to MSA) --- features of each DFG
  node (latency, resource type, mobility, fan-in/fan-out).
- A **dependency representation** (analogous to pair) --- features of each
  operation pair (data dependency, resource conflict, path criticality).

A scheduling network modeled on the Evoformer would maintain both
representations and let them cross-pollinate: knowing that operations A and B
are both on the critical path and both need a multiplier (pair info) constrains
what cycle each can be assigned to (operation info), and vice versa.

The **triangle update** maps directly to transitivity of scheduling constraints:
if *t(A) + d(A) <= t(B)* and *t(B) + d(B) <= t(C)*, then *t(A) + d(A) + d(B)
<= t(C)*. This is exactly an SDC constraint. An attention mechanism that
enforces this transitivity could learn to propagate constraint tightening more
aggressively than LP relaxation.

### 1.3  Rosetta Energy Functions

The Rosetta suite (Rohl et al., 2004; Alford et al., 2017) predicts and designs
protein structures using a **composite energy function**:

    E_total = w_vdw * E_vdw + w_hbond * E_hbond + w_elec * E_elec
              + w_sol * E_solvation + w_rama * E_Ramachandran + ...

Each term captures a different physical or statistical force. The weights *w_i*
are trained on a benchmark set of known structures. Rosetta uses **Monte Carlo
with simulated annealing** to minimise this energy, starting from fragment
insertions that sample the conformational space.

**HLS analogy.** The c2v scheduler currently minimises a single objective
(latency or II). A Rosetta-inspired approach would define a composite cost:

    C_total = w_lat * C_latency + w_area * C_area + w_power * C_power
              + w_route * C_routing_congestion + w_timing * C_timing_slack

and learn the weights from a benchmark set of successfully synthesised designs.
The Monte Carlo + simulated annealing search strategy handles the non-convexity
that pure LP cannot.

### 1.4  What c2v Should Steal

- **Funnel-shaped objective landscapes.** Redesign the scheduling cost function
  so that local perturbations (swapping adjacent operations, shifting a cycle)
  almost always move toward better solutions. This is the protein folding
  lesson: make the landscape smooth.
- **Dual-track attention.** If we ever add ML-guided scheduling, the Evoformer's
  dual-track (MSA + pair) architecture maps perfectly to (operation features +
  dependency features).
- **Composite energy functions.** Replace single-objective optimisation with
  Rosetta-style weighted multi-objective, with weights learned from benchmarks.

---

## 2  DNA Replication and Loop Pipelining

### 2.1  The Replication Fork as a Multi-Rate Pipeline

DNA replication is the canonical biological pipeline. At the replication fork:

1. **Helicase** (DnaB in E. coli) unwinds the double strand at ~1000 bp/s,
   acting as the pipeline clock.
2. **Leading-strand polymerase** (DNA Pol III) synthesises continuously in the
   5'->3' direction, directly coupled to helicase via the tau (tau) subunit of
   the clamp-loader complex (Georgescu et al., 2012).
3. **Lagging-strand polymerase** synthesises discontinuously in 1000--2000 bp
   **Okazaki fragments**, each requiring a new RNA primer (primase), clamp
   loading, synthesis, primer removal (RNase H / Pol I), and ligation (DNA
   ligase).

The leading strand runs at a steady rate; the lagging strand runs in bursts.
Studies show the lagging-strand polymerase synthesises DNA **faster** than the
leading strand (Georgescu et al., 2009, Cell; Yao et al., 2009) --- but it
periodically stalls for primer synthesis and clamp loading, so its average
throughput matches. This is a biological **multi-rate pipeline** with different
initiation intervals (IIs) per functional unit.

### 2.2  Okazaki Fragments as Modulo-Scheduled Iterations

Each Okazaki fragment is one iteration of the lagging-strand loop:

| Pipeline Stage         | Biological Process        | HLS Equivalent           |
|------------------------|---------------------------|--------------------------|
| Prologue               | Primase synthesises primer| Load operands from memory |
| Compute                | Pol III extends fragment  | ALU computation          |
| Epilogue               | RNase H removes primer,  | Write-back, register     |
|                        | ligase seals nick         | file update              |
| Stall / bubble         | Wait for new primer       | Pipeline bubble at II    |

The replication fork achieves **overlapping execution**: while one Okazaki
fragment is being synthesised, the primase is already laying down the primer for
the next one. This is precisely **modulo scheduling** (Rau, 1994): iterations
are overlapped with a fixed initiation interval.

The biological II is determined by the slowest stage --- typically primase,
which must synthesise a ~10 nt RNA primer. In E. coli, this takes ~1--2 seconds,
establishing an II of ~1500 bp / (1000 bp/s) = ~1.5s per Okazaki fragment
(Pandey et al., 2009).

### 2.3  Helicase-Polymerase Coupling as Resource Binding

The tau subunit physically tethers three Pol III cores to one clamp loader and
one helicase (McInerney et al., 2007). This is **resource binding**: three
functional units share one clock (helicase) and one resource allocator (clamp
loader). When the lagging-strand polymerase finishes a fragment, it **recycles**
--- releasing the old clamp and binding a new one at the next primer. This is
the biological equivalent of register file multiplexing.

Recent work shows that E. coli Pol III is **readily exchangeable** at the fork
(Lewis et al., 2017, eLife; Beattie et al., 2017) --- leading and lagging
strand synthesis may even be performed by different holoenzymes. This suggests
that tight coupling (binding a FU to a specific pipeline stage) is not always
optimal; a pool of interchangeable FUs with dynamic dispatch can be more robust.

### 2.4  Replication Fork Stalling and Pipeline Hazards

When the replication fork encounters a DNA lesion, the helicase can **decouple**
from the polymerase and run ahead, creating single-stranded DNA gaps (Yeeles &
Marians, 2011; Taylor & Yeeles, 2018). This is analogous to a pipeline that
continues issuing instructions past a cache miss, creating speculative state
that must be resolved later.

The cell resolves these gaps via **template switching** or **translesion
synthesis** --- analogous to pipeline recovery mechanisms (re-execution from
checkpoint, or degraded-accuracy execution).

### 2.5  What c2v Should Steal

- **Multi-rate II.** Allow different loop body regions to have different
  effective IIs, as the leading and lagging strands do. Currently modulo SDC
  scheduling enforces a single global II (Canis et al., 2013). Rational-II
  scheduling (Dai et al., 2017) partially addresses this.
- **Dynamic functional unit recycling.** Instead of statically binding FUs to
  pipeline stages, consider a pool of interchangeable FUs that are dynamically
  allocated --- biological replisome style.
- **Decoupled execution.** When one pipeline stage stalls, let others continue
  (helicase-polymerase decoupling). This maps to decoupled access-execute
  architectures in hardware (Smith, 1984).

---

## 3  Slime Mold and Ant Colony Optimisation for Design Space Exploration

### 3.1  Physarum polycephalum: Solving Network Optimisation Without a Brain

Tero et al. (2010, Science, vol. 327, pp. 439--442) demonstrated that the
slime mold Physarum polycephalum, placed on a map with food sources at
positions corresponding to Tokyo-area cities, grew a transport network
comparable in efficiency, fault tolerance, and cost to the actual Tokyo rail
system. The authors (Tero, Takagi, Saigusa, Ito, Bebber, Fricker, Yumiki,
Kobayashi, Nakagaki) formalised the mold's behaviour as a system of
differential equations governing tube conductance:

    dD_ij/dt = f(|Q_ij|) - alpha * D_ij

where *D_ij* is the conductance of tube *(i,j)*, *Q_ij* is the flux through it,
*f* is a monotonically increasing function, and *alpha* is a decay constant.
Tubes with high flux grow; tubes with low flux shrink. This is a **positive
feedback loop** that prunes the network to efficient paths.

The time complexity of Physarum-inspired algorithms is O(n^3) per iteration
(Zhang et al., 2014), dominated by solving the flow equations. Sun et al.
(2017, arXiv:1712.02910) provide a comprehensive review of Physarum-inspired
network optimisation algorithms and their convergence properties.

### 3.2  HLS Design Space Exploration as Network Optimisation

The HLS design space is a graph where:
- **Nodes** are design points (specific scheduling + binding + allocation
  decisions).
- **Edges** connect design points reachable by a single transformation (change
  one FU count, shift one operation by one cycle, etc.).
- **Objective** is to find the Pareto frontier of (latency, area, power).

Physarum's tube-conductance model maps directly: let the "conductance" of a
design-space edge be proportional to how often it appears in near-Pareto
solutions. Edges leading to good designs get reinforced; edges leading to bad
designs decay. The algorithm converges to a set of high-conductance paths
through the design space, each corresponding to a Pareto-optimal design.

### 3.3  Ant Colony Optimisation (ACO)

Dorigo (1992) introduced Ant Colony Optimisation, inspired by the pheromone
trails that real ants use to find shortest paths between nest and food source.
The formal algorithm:

1. *m* artificial ants construct solutions by traversing a construction graph.
2. Edge *(i,j)* has pheromone *tau_ij* and heuristic *eta_ij*. Ant *k* at node
   *i* chooses *j* with probability proportional to
   *tau_ij^alpha * eta_ij^beta*.
3. After all ants finish, pheromone evaporates: *tau_ij <- (1-rho) * tau_ij*.
4. Ants that found good solutions deposit pheromone on their edges:
   *tau_ij <- tau_ij + Delta_tau_ij*.

ACO has been applied directly to FPGA placement and routing:
- **FPGA placement:** Ant colony methods handle symmetry constraints while
  considering routing congestion (Sait et al., 2009, IEEE).
- **FPGA routing:** ACO solves routing with minimum channel width by
  transforming the geometric routing task into a Boolean satisfiability
  equation, achieving fewer tracks per channel than standard approaches
  (Chopra et al., 2014).

### 3.4  Multi-Objective ACO for Pareto Frontier Exploration

Pareto Ant Colony Optimisation (P-ACO) extends ACO to multi-objective
problems (Doerner et al., 2004, Annals of Operations Research). P-ACO
maintains a Pareto archive of non-dominated solutions and uses Pareto dominance
to guide pheromone updates. Key insight: instead of a single pheromone matrix,
P-ACO uses one pheromone matrix per objective, and ants probabilistically
weight the matrices.

For HLS design space exploration, this means maintaining separate pheromone
trails for latency, area, and power. An ant constructing a schedule would
consult all three trails, with weights drawn from a distribution that ensures
diverse Pareto coverage.

### 3.5  Physarum vs. ACO: Complementary Strengths

| Property                | Physarum               | ACO                    |
|-------------------------|------------------------|------------------------|
| Representation          | Continuous (flow)      | Discrete (graph walk)  |
| Feedback                | Flux-proportional      | Pheromone              |
| Exploration             | Parallel (all tubes)   | Sequential (ant paths) |
| Convergence             | Provable (Bonifaci     | Proven for TSP (Dorigo |
|                         | et al., 2012)          | & Stutzle, 2004)       |
| Multi-objective         | Natural (multi-source  | Requires P-ACO         |
|                         | flows)                 | extension              |

### 3.6  What c2v Should Steal

- **Physarum-style design space pruning.** Initialise a fully-connected design
  space graph; let tube conductances evolve over many scheduling runs; prune
  edges with near-zero conductance. The remaining graph is the "efficient
  design manifold."
- **ACO for binding.** Graph coloring for binding is NP-hard. ACO is well-suited
  to graph coloring problems and can explore the space of valid colorings more
  effectively than greedy heuristics.
- **P-ACO for Pareto-optimal exploration.** When the user specifies
  area/latency/power trade-offs, use multi-objective ACO to find the Pareto
  frontier rather than sweeping a single parameter.

---

## 4  Neural Circuit Topology and FPGA Resource Sharing

### 4.1  Retinal Center-Surround as Spatial Decorrelation

The vertebrate retina implements a classic signal-processing motif:
**center-surround receptive fields.** Retinal ganglion cells (RGCs) respond to
light in a central region and are inhibited by light in the surrounding annulus
(Kuffler, 1953). This is implemented by:

1. **Horizontal cells** in the outer plexiform layer provide **lateral
   inhibition** via GABAergic feedback onto photoreceptor terminals (Thoreson &
   Mangel, 2012).
2. **Amacrine cells** in the inner plexiform layer provide additional lateral
   inhibition onto bipolar cell terminals and RGC dendrites (Taylor & Smith,
   2011).

The purpose is **spatial decorrelation**: neighboring pixels in the visual field
are highly correlated, so transmitting raw pixel values wastes bandwidth. The
center-surround structure transmits the *difference* from the local mean,
compressing the signal. Barlow (1961) formalised this as the "efficient coding
hypothesis."

### 4.2  Lateral Inhibition as Resource Conflict Resolution

In FPGA binding, **resource sharing** means mapping multiple operations onto the
same functional unit (multiplexed in time). Two operations can share a FU if and
only if they are not active in the same clock cycle --- i.e., their **lifetimes
do not overlap.** This is the classic interval graph coloring problem.

Lateral inhibition provides a biological solution. Consider operations as
neurons and FUs as "channels." When operation A is bound to FU_1, it
**inhibits** nearby operations (in the DFG) from binding to FU_1 if their
lifetimes overlap. Operations that are far apart in the DFG (and hence likely
have non-overlapping lifetimes) are not inhibited, and can share the FU.

This maps to a neural network where:
- **Excitation** represents the affinity of an operation for a particular FU
  (matching resource type, minimising multiplexer cost).
- **Inhibition** represents resource conflict (overlapping lifetimes).
- The network settles to a stable state where each operation is bound to
  exactly one FU and no conflicts exist.

Hopfield & Tank (1985) showed that neural networks with lateral inhibition can
solve combinatorial optimisation problems (TSP). The same architecture can solve
the binding problem.

### 4.3  The Binding Problem in Neuroscience

In neuroscience, the **binding problem** asks: how does the brain combine
information from different sensory modalities (color, shape, motion) into a
unified percept? (Treisman & Gelade, 1980; Roskies, 1999). One proposed
solution is **temporal synchrony**: features belonging to the same object fire
in synchrony, while features of different objects fire asynchronously (Singer &
Gray, 1995; Engel et al., 2001).

In FPGA synthesis, the **binding problem** asks: how do we combine information
from different operations (adds, multiplies, loads) onto shared resources
(ALUs, multipliers, memory ports) without conflict? The solution is analogous:
operations belonging to the same time-step are "synchronous" and must not share
resources; operations in different time-steps are "asynchronous" and can share.

The graph coloring formulation of binding (Paulin & Knight, 1989) can be
interpreted as temporal synchrony: operations assigned the same "color" (FU)
must not be simultaneously active, just as features bound to the same object
must fire in synchrony and *not* overlap with features of other objects.

### 4.4  Cortical Column Architecture and Hierarchical Binding

The neocortex is organised into **cortical columns** (Mountcastle, 1997) ---
vertical units ~300 um in diameter that process a specific feature or spatial
location. Within a column, six layers perform different computations:

- Layer 4 receives thalamic input (raw data).
- Layers 2/3 perform lateral integration (neighboring columns).
- Layer 5 generates output to subcortical structures.
- Layer 6 provides feedback to thalamus.

This hierarchical structure maps to a multi-level FPGA binding strategy:
- **Level 1 (intra-basic-block):** Bind operations within a single basic block
  using local conflict graphs --- analogous to intra-column processing.
- **Level 2 (inter-basic-block):** Share FUs across basic blocks that are
  mutually exclusive (different branches of an if/else) --- analogous to
  inter-column lateral inhibition.
- **Level 3 (inter-function):** Share FUs across different function calls ---
  analogous to cortical feedback loops.

### 4.5  What c2v Should Steal

- **Lateral inhibition for binding.** Replace greedy graph coloring with a
  neural-network-inspired iterative relaxation where FU assignments inhibit
  conflicting assignments and excite compatible ones.
- **Hierarchical binding.** Bind at multiple granularities (intra-BB,
  inter-BB, inter-function) like cortical columns process at multiple scales.
- **Temporal synchrony detection.** Use DFG timing analysis to identify
  operation groups that are "synchronous" (same cycle) and treat them as a
  binding exclusion set, analogous to feature binding via synchrony.

---

## 5  Genetic Regulatory Networks and DFG Optimisation

### 5.1  Gene Expression as a Directed Acyclic Graph

A **gene regulatory network** (GRN) describes how genes activate or inhibit
each other's expression. At the simplest level, each gene is a node and each
regulatory interaction is a directed edge. When feedback loops are removed (or
unrolled over time), the resulting graph is a DAG --- exactly the structure of
a data-flow graph in HLS.

Stuart Kauffman (1969) introduced **Boolean networks** as a model of GRNs: each
gene is a binary variable (on/off) updated by a Boolean function of its
regulators. A Boolean network with *n* genes has 2^n possible states. Since the
state space is finite, trajectories must eventually reach a **fixed point** or
**cycle** --- called an **attractor**. Kauffman proposed that attractors
correspond to cell types: a human has ~200 cell types and ~20,000 genes,
consistent with the prediction that networks at the "edge of chaos" (K ~ 2
inputs per gene) have ~sqrt(n) attractors (Kauffman, 1993).

### 5.2  Boolean Network Dynamics and Pass Ordering

In a Boolean network, the **update order** matters. Synchronous updates (all
genes update simultaneously) give different dynamics than asynchronous updates
(one gene at a time, chosen randomly or by priority).

In HLS, **pass ordering** is the analogous problem: the order in which
optimisation passes run (CSE, reassociation, strength reduction, width
narrowing, tree balancing) affects the final result. Different orderings can
reach different "attractors" --- local optima in the design space.

Stochastic Boolean Networks (Liang et al., 2012, BMC Systems Biology) model
this uncertainty by assigning probabilities to the Boolean functions. The
resulting dynamics explore the attractor landscape more thoroughly than
deterministic updates. For c2v, this suggests **stochastic pass ordering**:
instead of a fixed pass sequence, randomly permute the passes and run multiple
times, keeping the best result. The probability of choosing a particular
ordering can be biased by past success (analogous to the SBN's probability
distribution).

### 5.3  Gillespie Stochastic Simulation Algorithm

The Gillespie algorithm (Gillespie, 1977, J. Phys. Chem.) simulates the
stochastic dynamics of chemical reaction networks:

1. Compute the propensity *a_i* of each reaction *i* (proportional to
   reactant concentrations and rate constant).
2. Draw the time to next reaction from an exponential distribution with rate
   *a_0 = sum(a_i)*.
3. Choose which reaction fires with probability *a_i / a_0*.
4. Update molecular counts. Repeat.

This is a **continuous-time Markov chain** simulation. Weidmann et al. (2007,
arXiv:0704.3406) applied Gillespie-style stochastic simulation to Boolean
networks, modeling gene expression as a chemical reaction.

**HLS analogy.** Each optimisation pass can be modeled as a "reaction" that
transforms the DFG. The propensity of a pass is proportional to how much
improvement it is likely to yield (estimated from the current DFG structure).
The Gillespie algorithm then selects which pass to run next, with
higher-propensity passes firing more frequently. This is **adaptive pass
ordering** guided by stochastic simulation.

### 5.4  Network Motifs and DFG Patterns

Alon (2007, "An Introduction to Systems Biology") catalogued recurring **network
motifs** in GRNs:

- **Feed-forward loop (FFL):** Gene A activates both B and C; B also activates
  C. Provides a delay filter --- C only turns on if A is persistently active.
- **Single-input module (SIM):** One gene regulates a set of targets with
  different thresholds, creating a temporal program of activation.
- **Dense overlapping regulons (DOR):** Multiple regulators with overlapping
  target sets.

These motifs have DFG counterparts:
- **FFL** corresponds to a computation where an intermediate result is used
  both directly and through a derived value --- common in polynomial evaluation.
  The "delay filter" property means the FFL is naturally pipelineable.
- **SIM** corresponds to a broadcast from one register to multiple FUs with
  different latencies --- a common pattern in tree-structured reductions.
- **DOR** corresponds to shared sub-expressions --- the target of CSE.

### 5.5  What c2v Should Steal

- **Stochastic pass ordering a la Gillespie.** Assign propensities to passes
  based on estimated improvement. Use the Gillespie algorithm to select which
  pass to run next. Run multiple stochastic trajectories and keep the best.
- **Attractor analysis for pass convergence.** Model the pass pipeline as a
  Boolean network; find its attractors to identify which pass orderings
  converge to the same fixed point (equivalent DFG), avoiding redundant work.
- **Network motif detection.** Scan the DFG for FFL, SIM, and DOR motifs.
  Apply motif-specific optimisations (pipeline FFLs, broadcast SIMs, CSE DORs).

---

## 6  Immune System and Optimisation Pass Ordering

### 6.1  V(D)J Recombination: Combinatorial Diversity Generation

The adaptive immune system generates ~10^13 unique antibody specificities from
a relatively small genome (~400 gene segments) via **V(D)J recombination**
(Tonegawa, 1983, Nobel Prize). The heavy-chain locus contains:

- ~40 Variable (V) segments
- ~23 Diversity (D) segments
- ~6 Joining (J) segments

Recombination randomly selects one V, one D, and one J, concatenating them with
**junctional diversification** (N-nucleotide addition by terminal
deoxynucleotidyl transferase, P-nucleotide addition from hairpin opening).
Combined with light-chain pairing, this yields a combinatorial space exceeding
10^16 unique antibodies (Murphy & Weaver, 2016, Janeway's Immunobiology).

**HLS analogy.** c2v has ~10 optimisation passes (CSE, reassociation, strength
reduction, width narrowing, tree balancing, etc.). Each pass has parameters
(thresholds, iteration counts, scope). The space of pass orderings and
parameterisations is the "immune repertoire" of the compiler. V(D)J-style
recombination would randomly assemble pass sequences from a library of pass
"segments," with junctional diversification adding random parameter
perturbations at the boundaries.

### 6.2  Clonal Selection and Affinity Maturation

Once a naive B cell encounters an antigen, it undergoes **clonal expansion**:
the B cell divides rapidly, producing many copies. During division, **somatic
hypermutation** (SHM) introduces point mutations into the antibody gene at a
rate of ~10^-3 per base pair per division (Rajewsky, 1996). B cells whose
mutated antibodies bind the antigen more tightly are **positively selected**
in germinal centers; those with worse binding undergo apoptosis.

This is a **local search with adaptive mutation rate**: high-affinity
antibodies mutate less per division but divide more (Victora & Nussenzweig,
2012, Annual Review of Immunology). The mutation rate is **anti-correlated**
with fitness --- exactly the strategy needed for effective simulated annealing.

De Castro & Von Zuben (2002, IEEE Trans. Evol. Computation) formalised this as
the **CLONALG** algorithm:

1. Generate initial population of candidate solutions (antibodies).
2. Evaluate fitness (affinity to antigen / quality of compiled design).
3. Clone the best solutions proportionally to their fitness.
4. Hypermutate clones inversely proportionally to their fitness (good solutions
   get small perturbations; bad solutions get large perturbations).
5. Select the best mutants to replace the worst members of the population.
6. Introduce random new solutions (receptor editing) to maintain diversity.

### 6.3  Immune Memory and Compilation Caching

After infection is cleared, a subset of high-affinity B cells differentiate
into **memory B cells** that persist for decades. Upon re-encounter with the
same antigen, memory cells mount a faster, stronger response.

**HLS analogy.** When c2v compiles a function, the best pass ordering and
parameterisation should be cached. When a similar function is encountered
later (similar DFG structure, similar resource constraints), the cached
ordering serves as the starting point --- analogous to an immune memory
response that doesn't need to restart from naive B cells.

### 6.4  Danger Theory and Adaptive Compilation

Matzinger (2002, Science) proposed the **Danger Theory**: the immune system
does not simply distinguish self from non-self, but rather responds to
**danger signals** --- indicators of cellular damage. This shifts the trigger
from "foreign" to "harmful."

**HLS analogy.** Instead of running all passes on every function
(distinguishing "optimised" from "unoptimised"), c2v should detect **danger
signals** in the DFG: critical-path length exceeding target latency, resource
utilisation exceeding target, timing violations. Only when danger signals are
present should aggressive optimisation (hypermutation) be triggered. This
avoids wasting compilation time on functions that already meet targets.

### 6.5  What c2v Should Steal

- **CLONALG for pass ordering.** Generate a population of pass orderings;
  evaluate each by compiling and measuring QoR. Clone the best, hypermutate
  inversely to fitness, select survivors. Converges to good orderings in
  O(population * generations) compilations.
- **Immune memory for compilation caching.** Cache successful pass orderings
  keyed by DFG fingerprint (graph hash). On cache hit, start from the cached
  ordering instead of the default.
- **Danger-signal triggered optimisation.** Only run expensive passes when the
  current design violates a target metric. If latency is fine but area is too
  high, only run area-reducing passes.

---

## 7  Evolutionary Algorithms for HLS

### 7.1  Cartesian Genetic Programming (CGP)

Julian Miller introduced **Cartesian Genetic Programming** in 1999--2000
(Miller & Thomson, 2000, EuroGP) as a variant of genetic programming where
programs are represented as directed acyclic graphs on a Cartesian grid of
*r* rows and *c* columns. Each node has a function (AND, OR, XOR, MUX, ADD,
MUL, ...) and *n* inputs drawn from previous columns or primary inputs.

Key properties of CGP relevant to HLS:

- **Implicit redundancy.** Not all nodes need to be connected to the output.
  Disconnected nodes are **neutral** with respect to fitness but provide
  genetic material for future mutations. This is the CGP equivalent of **dead
  code** that can be revived by later mutations --- a powerful mechanism for
  escaping local optima.

- **Mutation-only evolution.** CGP traditionally uses a (1+lambda) evolution
  strategy with point mutation and no crossover. Miller & Smith (2006) showed
  that crossover is detrimental in CGP because it disrupts the positional
  encoding. Goldman & Punch (2013, GECCO) confirmed this result and showed
  that CGP's neutral drift through disconnected nodes is the primary driver
  of exploration.

- **Circuit evolution results.** CGP has produced circuits that are **more
  efficient** than human designs:
  - 2-bit multiplier: 7 gates vs. 10--11 for conventional designs (Miller
    et al., 2000).
  - 3-bit multiplier: 20% fewer gates than best known conventional design
    (Vasicek & Sekanina, 2011, IEEE Trans. Evol. Computation).
  - Even-parity circuits: CGP finds solutions with fewer gates than
    hand-optimised designs (Miller, 2011).

### 7.2  Evolvable Hardware on FPGAs

**Intrinsic evolution** runs the evolutionary algorithm on the FPGA itself,
evaluating fitness by reconfiguring the device and measuring actual performance.
Thompson (1997) famously evolved a tone discriminator on a Xilinx XC6216 FPGA
that used analog effects of the silicon --- effects that no human designer
would exploit.

More recent work:
- Vasicek & Sekanina (2008, 2011) evolved real-time image filters on FPGAs
  using CGP, achieving adaptive noise removal that outperforms fixed filters.
- Harding & Miller (2009) demonstrated evolving digital circuits with up to
  hundreds of inputs and thousands of gates, scaling CGP to industrially
  relevant problem sizes.
- Salvador et al. (2020) used evolvable hardware architectures on FPGAs for
  side-channel security, evolving countermeasures that are difficult to reverse
  engineer.

### 7.3  Multi-Objective Evolutionary Algorithms for HLS DSE

The HLS design space exploration problem is naturally multi-objective: minimise
latency, area, and power simultaneously. Key algorithms:

- **NSGA-II** (Deb et al., 2002, IEEE Trans. Evol. Computation): Non-dominated
  Sorting Genetic Algorithm. Uses Pareto dominance ranking and crowding distance
  to maintain a diverse Pareto front. NSGA-II has been applied to HLS DSE by
  Schafer & Wang (2019).

- **SPEA2** (Zitzler et al., 2001): Strength Pareto Evolutionary Algorithm.
  Uses an external archive of non-dominated solutions and a fine-grained
  fitness assignment based on dominance strength.

- **MOEA/D** (Zhang & Li, 2007, IEEE Trans. Evol. Computation): Multi-objective
  Evolutionary Algorithm based on Decomposition. Decomposes the multi-objective
  problem into scalar sub-problems and solves them simultaneously with
  information sharing between neighboring sub-problems.

Pilato & Ferrandi (2013) applied multi-objective GAs to concurrent scheduling
and binding in HLS, exploring the Pareto frontier of area vs. latency. Their
approach simultaneously optimises both sub-problems, avoiding the phase-ordering
problem that plagues sequential scheduling-then-binding flows.

### 7.4  Recent Trends: ML + Evolution Hybrids (2020--2026)

The frontier has moved toward hybrid approaches combining evolutionary
algorithms with machine learning:

- **GNN-guided DSE.** Wu et al. (2022, DAC) use graph neural networks to
  predict the quality of HLS designs from the DFG, then use the predictions as
  a surrogate fitness function for evolutionary search, reducing the number of
  actual synthesis runs by 10--100x.

- **Reinforcement learning for HLS.** Ironman (Roy et al., 2021,
  arXiv:2102.08138) uses GNN-assisted reinforcement learning for design space
  exploration, learning a policy that navigates the design space more
  efficiently than evolutionary search.

- **LLM-guided DSE.** Intelligent4DSE (Yu et al., 2025, arXiv:2504.19649)
  combines GNNs with large language models to guide HLS design space
  exploration, using the LLM's code understanding to generate better design
  candidates.

- **SEER** (Coward et al., 2023, arXiv:2308.07654) uses e-graph rewriting
  within MLIR to find optimal HLS transformations, combining equality
  saturation (a technique from compiler optimisation) with hardware-aware cost
  models.

- **Hardware-aware NAS.** Neural Architecture Search with hardware cost models
  (Benmeziane et al., 2021) jointly optimises neural network architecture and
  hardware accelerator design on FPGAs.

### 7.5  CGP-SA: Simulated Annealing Meets CGP

Recent work (Vassilev et al., 2024, Springer LNCS) integrates a simulated
annealing mechanism into CGP's selection operator, replacing the standard
(1+lambda) strategy. CGP-SA allows temporarily accepting worse solutions with
a probability that decreases over time, helping escape local optima. Results
on multiplier and adder circuits show improved convergence and final solution
quality compared to standard CGP.

### 7.6  What c2v Should Steal

- **CGP for peephole optimisation.** Use CGP to evolve optimal implementations
  of small combinational sub-circuits (e.g., multiplier trees, reduction
  networks, address decoders) within the Verilog emitter. The evolved circuits
  can be more efficient than template-based code generation.

- **Neutral drift for escaping local optima.** CGP's neutral drift mechanism
  --- carrying "dead" nodes that can be revived by later mutations --- maps to
  maintaining "dormant" optimisation passes or alternative DFG fragments that
  are not currently active but can be activated by pass reordering.

- **Surrogate-assisted evolution.** Train a lightweight model (GNN or random
  forest) to predict synthesis QoR from DFG features. Use this as a surrogate
  fitness function for evolutionary DSE, evaluating thousands of design points
  without running actual synthesis. Only run Vivado/Yosys for the top
  candidates.

- **NSGA-II for multi-objective exploration.** When the user specifies
  trade-off preferences (e.g., "minimise area, latency < 10 cycles"), use
  NSGA-II to find the Pareto frontier and present the best designs.

---

## 8  Synthesis: A Unified Biological Framework for c2v

### 8.1  Mapping Biological Systems to the c2v Pipeline

| c2v Pipeline Stage | Biological System           | Algorithm to Steal               |
|--------------------|-----------------------------|----------------------------------|
| **Scheduling**     | Protein folding             | Funnel-shaped energy landscape,  |
|                    |                             | Rosetta composite cost function  |
| **Loop pipelining**| DNA replication fork        | Multi-rate II, dynamic FU        |
|                    |                             | recycling, decoupled execution   |
| **DSE**            | Physarum / ACO              | Tube-conductance pruning,        |
|                    |                             | P-ACO Pareto exploration         |
| **Binding**        | Neural lateral inhibition   | Inhibition-based conflict        |
|                    |                             | resolution, hierarchical binding |
| **DFG optimisation**| Gene regulatory networks   | Stochastic pass ordering,        |
|                    |                             | network motif detection          |
| **Pass ordering**  | Immune clonal selection     | CLONALG, immune memory caching,  |
|                    |                             | danger-signal triggered passes   |
| **Code generation**| Evolvable hardware / CGP    | Peephole circuit evolution,      |
|                    |                             | neutral drift, surrogate fitness |

### 8.2  Implementation Priority

Ranked by expected impact-to-effort ratio for c2v:

1. **Stochastic pass ordering (Gillespie / CLONALG).** Low implementation
   effort --- c2v already has the pass infrastructure. Just wrap the existing
   passes in a stochastic selection loop. Expected improvement: 5--15% QoR on
   complex benchmarks where pass ordering matters.

2. **Composite cost function (Rosetta-style).** Moderate effort --- requires
   defining cost terms and learning weights. But the payoff is large: proper
   multi-objective optimisation is essential for beating Vitis, which uses a
   fixed heuristic weighting.

3. **ACO for binding.** Moderate effort --- replace the graph coloring heuristic
   with ACO. Expected improvement: better multiplexer sharing in designs with
   many compatible FUs.

4. **Network motif detection in DFG.** Low effort --- pattern matching on the
   DFG. Apply specialised optimisations to detected motifs (FFLs, SIMs, DORs).

5. **CGP peephole optimisation.** Higher effort but potentially game-changing
   for arithmetic-heavy designs. Run offline, cache results, emit evolved
   circuits as Verilog templates.

6. **Surrogate-assisted evolution for DSE.** High effort (requires training
   data from synthesis runs), but provides the most systematic approach to
   exploring the design space.

7. **Multi-rate II scheduling.** High effort --- requires extending the SDC
   formulation. But enables designs that are fundamentally impossible with
   single-II scheduling.

### 8.3  Cross-Pollination Between Biological Domains

The biological systems are not independent. They share common mathematical
structures:

- **Energy minimisation** unifies protein folding (free energy), scheduling
  (latency), and ACO (path cost). All three use gradient-like search on
  rugged landscapes.

- **Positive feedback with decay** unifies Physarum (tube conductance), ACO
  (pheromone), and immune affinity maturation (B cell clonal expansion). All
  three reinforce good solutions and let bad solutions decay.

- **Combinatorial recombination** unifies V(D)J recombination, CGP crossover
  (or its absence), and GRN Boolean function selection. All three generate
  diversity by combining modular components.

- **Hierarchical processing** unifies cortical columns (layers 1--6), DNA
  replication (leading/lagging/Okazaki), and CGP (rows/columns of the
  Cartesian grid). All three decompose a global problem into nested local
  sub-problems.

A unified framework would model the c2v compilation pipeline as a
**biological system** where:
- The DFG is the "genome."
- Optimisation passes are "enzymes."
- The scheduler is the "ribosome" (translates the genome into a structure).
- The emitter is "protein folding" (the structure acquires its final 3D form).
- The design space is the "fitness landscape."
- Compilation is "evolution" across generations of design variants.

---

## References

### Protein Folding and Scheduling

- Levinthal, C. (1969). How to fold graciously. Mossbauer Spectroscopy in
  Biological Systems, pp. 22--24.
- Dill, K. A. & Chan, H. S. (1997). From Levinthal to pathways to funnels.
  Nature Structural Biology, 4(1), 10--19.
- Onuchic, J. N., Luthey-Schulten, Z. & Wolynes, P. G. (1997). Theory of
  protein folding: the energy landscape perspective. Annual Review of Physical
  Chemistry, 48, 545--600.
- Berger, B. & Leighton, T. (1998). Protein folding in the hydrophobic-
  hydrophilic (HP) model is NP-complete. Journal of Computational Biology,
  5(1), 27--40.
- Crescenzi, P., Goldman, D., Papadimitriou, C., Piccolboni, A. & Yannakakis,
  M. (1998). On the complexity of protein folding. Journal of Computational
  Biology, 5(3), 423--465.
- Rohl, C. A., Strauss, C. E., Misura, K. M. & Baker, D. (2004). Protein
  structure prediction using Rosetta. Methods in Enzymology, 383, 66--93.
- Alford, R. F., Leaver-Fay, A., Jeliazkov, J. R. et al. (2017). The Rosetta
  all-atom energy function for macromolecular modeling and design. Journal of
  Chemical Theory and Computation, 13(6), 3031--3048.
- Jumper, J. et al. (2021). Highly accurate protein structure prediction with
  AlphaFold. Nature, 596, 583--589.

### DNA Replication and Loop Pipelining

- Rau, B. R. (1994). Iterative modulo scheduling: an algorithm for software
  pipelining loops. MICRO-27, pp. 63--74.
- Georgescu, R. E., Kurth, I. & O'Donnell, M. E. (2012). Single-molecule
  studies reveal the function of a third polymerase in the replisome. Nature
  Structural & Molecular Biology, 19(1), 113--116.
- Georgescu, R. E. et al. (2009). Mechanism of polymerase collision release
  from sliding clamps on the lagging strand. EMBO Journal, 28, 2981--2991.
- Pandey, M. et al. (2009). Coordinating DNA replication by means of priming
  loop and differential synthesis rate. Nature, 462, 940--943.
- Lewis, J. S. et al. (2017). Single-molecule visualization of fast
  polymerase turnover in the bacterial replisome. eLife, 6, e23932.
- Beattie, T. R. et al. (2017). Frequent exchange of the DNA polymerase
  during bacterial chromosome replication. eLife, 6, e21763.
- Yeeles, J. T. P. & Marians, K. J. (2011). The Escherichia coli replisome
  is inherently DNA damage tolerant. Science, 334, 235--238.
- Cong, J. & Zhang, Z. (2006). An efficient and versatile scheduling
  algorithm based on SDC formulation. DAC, pp. 433--438.
- Canis, A. et al. (2013). LegUp: An open-source high-level synthesis tool
  for FPGA-based processor/accelerator systems. ACM TECS, 13(2), 24.
- Dai, S. et al. (2017). From multi-to fractional-integer-II modulo
  scheduling for high-level synthesis. ICCAD.

### Slime Mold and ACO

- Tero, A., Takagi, S., Saigusa, T. et al. (2010). Rules for biologically
  inspired adaptive network design. Science, 327(5964), 439--442.
- Sun, Y. et al. (2017). Physarum-inspired network optimization: a review.
  arXiv:1712.02910.
- Zhang, X. et al. (2014). An improved Physarum polycephalum algorithm for
  the shortest path problem. Scientific World Journal, 487069.
- Bonifaci, V., Mehlhorn, K. & Varber, G. (2012). Physarum can compute
  shortest paths. Journal of Theoretical Biology, 309, 121--133.
- Dorigo, M. (1992). Optimization, learning and natural algorithms. PhD
  thesis, Politecnico di Milano.
- Dorigo, M. & Stutzle, T. (2004). Ant Colony Optimization. MIT Press.
- Doerner, K. F., Gutjahr, W. J., Hartl, R. F. et al. (2004). Pareto ant
  colony optimization: a metaheuristic approach to multiobjective portfolio
  selection. Annals of Operations Research, 131, 79--99.
- Sait, S. M. et al. (2009). Ant colony optimization for symmetrical FPGA
  placement. IEEE Symposium on CIISP.
- Chopra, K. et al. (2014). Ant colony optimization approach for solving
  FPGA routing with minimum channel width.

### Neural Circuits and Binding

- Kuffler, S. W. (1953). Discharge patterns and functional organization of
  mammalian retina. Journal of Neurophysiology, 16(1), 37--68.
- Barlow, H. B. (1961). Possible principles underlying the transformation
  of sensory messages. In Sensory Communication, MIT Press, pp. 217--234.
- Hopfield, J. J. & Tank, D. W. (1985). Neural computation of decisions in
  optimization problems. Biological Cybernetics, 52, 141--152.
- Treisman, A. M. & Gelade, G. (1980). A feature-integration theory of
  attention. Cognitive Psychology, 12(1), 97--136.
- Singer, W. & Gray, C. M. (1995). Visual feature integration and the
  temporal correlation hypothesis. Annual Review of Neuroscience, 18,
  555--586.
- Engel, A. K., Fries, P. & Singer, W. (2001). Dynamic predictions: oscillations
  and synchrony in top-down processing. Nature Reviews Neuroscience, 2, 704--716.
- Roskies, A. L. (1999). The binding problem. Neuron, 24, 7--9.
- Mountcastle, V. B. (1997). The columnar organization of the neocortex.
  Brain, 120, 701--722.
- Paulin, P. G. & Knight, J. P. (1989). Force-directed scheduling for the
  behavioral synthesis of ASICs. IEEE TCAD, 8(6), 661--679.

### Gene Regulatory Networks

- Kauffman, S. A. (1969). Metabolic stability and epigenesis in randomly
  constructed genetic nets. Journal of Theoretical Biology, 22(3), 437--467.
- Kauffman, S. A. (1993). The Origins of Order. Oxford University Press.
- Gillespie, D. T. (1977). Exact stochastic simulation of coupled chemical
  reactions. Journal of Physical Chemistry, 81(25), 2340--2361.
- Liang, S. et al. (2012). Stochastic Boolean networks: an efficient approach
  to modeling gene regulatory networks. BMC Systems Biology, 6, 113.
- Alon, U. (2007). An Introduction to Systems Biology: Design Principles of
  Biological Circuits. CRC Press.

### Immune System

- Tonegawa, S. (1983). Somatic generation of antibody diversity. Nature,
  302, 575--581.
- Murphy, K. & Weaver, C. (2016). Janeway's Immunobiology. 9th edition.
  Garland Science.
- Rajewsky, K. (1996). Clonal selection and learning in the antibody system.
  Nature, 381, 751--758.
- Victora, G. D. & Nussenzweig, M. C. (2012). Germinal centers. Annual
  Review of Immunology, 30, 429--457.
- De Castro, L. N. & Von Zuben, F. J. (2002). Learning and optimization
  using the clonal selection principle. IEEE Trans. Evol. Computation, 6(3),
  239--251.
- Matzinger, P. (2002). The danger model: a renewed sense of self. Science,
  296(5566), 301--305.

### Evolutionary Algorithms and HLS

- Miller, J. F. & Thomson, P. (2000). Cartesian genetic programming. EuroGP
  2000, LNCS 1802, pp. 121--132.
- Miller, J. F. & Smith, S. L. (2006). Redundancy and computational
  efficiency in Cartesian genetic programming. IEEE Trans. Evol. Computation,
  10(2), 167--174.
- Goldman, B. W. & Punch, W. F. (2013). Length bias and search limitations
  in Cartesian genetic programming. GECCO, pp. 933--940.
- Thompson, A. (1997). An evolved circuit, intrinsic in silicon, entwined
  with physics. Evolvable Systems: From Biology to Hardware, LNCS 1259,
  pp. 390--405.
- Vasicek, Z. & Sekanina, L. (2011). Formal verification of candidate
  solutions for post-synthesis evolutionary optimization in evolvable
  hardware. Genetic Programming and Evolvable Machines, 12(3), 305--327.
- Salvador, R. et al. (2020). Evolvable hardware architectures on FPGA for
  side-channel security.
- Deb, K., Pratap, A., Agarwal, S. & Meyarivan, T. (2002). A fast and
  elitist multiobjective genetic algorithm: NSGA-II. IEEE Trans. Evol.
  Computation, 6(2), 182--197.
- Zitzler, E., Laumanns, M. & Thiele, L. (2001). SPEA2: Improving the
  strength Pareto evolutionary algorithm. TIK-Report 103, ETH Zurich.
- Zhang, Q. & Li, H. (2007). MOEA/D: A multiobjective evolutionary algorithm
  based on decomposition. IEEE Trans. Evol. Computation, 11(6), 712--731.
- Pilato, C. & Ferrandi, F. (2013). Bambu: A modular framework for the
  high level synthesis of memory-intensive applications. FPL.
- Roy, S. K. et al. (2021). IronMan: GNN-assisted design space exploration
  in high-level synthesis via reinforcement learning. arXiv:2102.08138.
- Coward, S. et al. (2023). SEER: Super-optimization explorer for HLS using
  e-graph rewriting with MLIR. arXiv:2308.07654.
- Wu, N. et al. (2022). A graph deep learning framework for high-level
  synthesis design space exploration. DAC.
- Yu, Q. et al. (2025). Intelligent4DSE: Optimizing high-level synthesis
  design space exploration with graph neural networks and large language
  models. arXiv:2504.19649.
- Garey, M. R. & Johnson, D. S. (1979). Computers and Intractability: A
  Guide to the Theory of NP-Completeness. W. H. Freeman.

---

*Document prepared for c2v research. Last updated June 2026.*
