# Mathematics of Optimal Scheduling for HLS Compilers

## Context and Motivation

c2v compiles restricted C (no heap, no recursion, fixed-size arrays, no pointer
aliasing) to synthesisable Verilog. The scheduler takes a control-data flow graph
(CDFG) and assigns every operation to a clock cycle, subject to dependency,
resource, and timing constraints. We use **SDC (System of Difference Constraints)
scheduling** with an LP solver. The strategic goal is to **beat Xilinx Vitis HLS**
on quality of results for the restricted-C subset we support.

This document surveys the mathematical foundations that underlie optimal
scheduling, identifies which structural properties of restricted-C DFGs we can
exploit, and catalogues the state of the art through 2026.

---

## 1  Lattice Theory --- Schedules as Linear Extensions of Dependency Posets

### 1.1  The Schedule Space

A data-flow graph (DFG) with *n* operations and dependency edges induces a
**partial order** (poset) *P = (V, ≤)*. A **valid schedule** is a mapping
*σ : V → {0, 1, ..., L}* that respects the partial order --- i.e., if *u ≤ v*
then *σ(u) ≤ σ(v) - δ(u,v)* where *δ* encodes latency of operator *u*.

When all latencies are unit-weight and we ignore resource constraints, valid
schedules are exactly the **linear extensions** of the poset *P*: total orders
consistent with the partial order. The set of all linear extensions *L(P)*
therefore characterises the full space of legal schedules.

**Key identity.** Linear extensions of a poset *P* correspond to maximal chains
in the **distributive lattice** *J(P)* of order ideals (downsets) of *P*, by
Birkhoff's representation theorem. The lattice *J(P)* is the schedule lattice:
its elements are the sets of operations that *could* have completed by some
time-step, and its covering relations correspond to scheduling one additional
operation.

### 1.2  Brightwell--Winkler: Counting is #P-Complete

**Theorem (Brightwell & Winkler, 1991).** Computing *|L(P)|* --- the number of
linear extensions of a finite poset *P* --- is **#P-complete**.

This result has far-reaching consequences:

- **Volume computation.** Computing the volume of a rational polytope is
  #P-hard, since the order polytope *O(P)* has volume proportional to
  *|L(P)| / n!* (Stanley, 1986).
- **Average height.** Determining the expected position of an element across
  uniformly random linear extensions is #P-complete.
- **Comparison probability.** The probability that element *x* precedes element
  *y* in a random linear extension is #P-complete to compute exactly.

**Citation.** G. Brightwell and P. Winkler, "Counting Linear Extensions,"
*Order*, vol. 8, no. 3, pp. 225--242, 1991.

### 1.3  Restricted Posets: Height-Two and Dimension-Two

Brightwell and Winkler conjectured in 1991 that counting linear extensions
remains #P-complete even for posets of **height two** (longest chain has two
elements). This conjecture stood open for 27 years until resolved by Dittmer
and Pak (2018), who proved:

**Theorem (Dittmer & Pak, 2018).** Counting linear extensions of height-two
posets is #P-complete. The result extends to incidence posets of graphs and
posets of dimension two.

**Citation.** S. Dittmer and I. Pak, "Counting Linear Extensions of Restricted
Posets," *Electronic Journal of Combinatorics*, vol. 27, no. 4, P4.48, 2020.
arXiv:1802.06312.

### 1.4  Practical Implications for c2v

Since exact enumeration of the schedule space is intractable, we cannot
brute-force the optimal schedule. However:

1. **MCMC sampling** of linear extensions (via promotion/demotion chains on
   *J(P)*) can estimate schedule quality distributions. Talvitie et al. showed
   that adaptive relaxation Monte Carlo outperforms plain MCMC for counting
   linear extensions in practice.
2. **Stanley's P-partition theory** connects linear extensions to symmetric
   functions and generating functions, which may yield closed-form mobility
   estimates for structured DFGs.
3. For our restricted-C DFGs (no recursion, structured control flow), the
   posets have bounded width and height determined by the longest dependency
   chain, making the effective schedule space much smaller than the worst case.

---

## 2  Matroid Theory --- When Greedy Gives Optimal Schedules

### 2.1  Background: Matroids and the Edmonds--Rado Theorem

A **matroid** *M = (E, I)* is a pair of a ground set *E* and a family *I* of
"independent" subsets satisfying:

1. *∅ ∈ I* (empty set is independent).
2. If *A ∈ I* and *B ⊆ A*, then *B ∈ I* (hereditary / downward-closed).
3. If *A, B ∈ I* and *|A| < |B|*, then *∃ e ∈ B \ A* s.t. *A ∪ {e} ∈ I*
   (exchange / augmentation axiom).

**Theorem (Edmonds, 1971; Rado, 1957).** The greedy algorithm finds a
maximum-weight basis of a matroid. Conversely, if the greedy algorithm is
optimal for every weight function on a hereditary set system, then the system
is a matroid.

This is the deepest known characterisation of when greedy works.

### 2.2  Are Restricted-C DFGs Matroids?

Consider the scheduling problem: ground set = operations, a set *S* is
"independent" if *S* can be feasibly scheduled in ≤ *k* cycles for some bound
*k*. Does this satisfy the matroid axioms?

**No, in general.** The exchange axiom fails for resource-constrained scheduling.
Consider two independent sets of operations that both fit in *k* cycles. Taking
an element from the larger set and adding it to the smaller may violate resource
constraints at some cycle, breaking feasibility. The resource-constrained
scheduling problem is **NP-hard** (Garey & Johnson, 1975, by reduction from
3-Partition), which already implies the feasible sets cannot form a matroid
(since matroid optimisation is polynomial).

**Citation.** M. R. Garey and D. S. Johnson, "Complexity Results for
Multiprocessor Scheduling under Resource Constraints," *SIAM Journal on
Computing*, vol. 4, no. 4, pp. 397--411, 1975.

### 2.3  Greedoids and Antimatroids

While DFG scheduling is not a matroid problem, there is a natural **greedoid**
structure. A **greedoid** relaxes the hereditary property of matroids to require
only that independent sets be accessible (reachable by a sequence of
single-element additions from the empty set). The relevant subclass is:

**Antimatroids** (equivalently, **convex geometries**). The set of all valid
partial schedules (prefixes of linear extensions) forms an antimatroid. Boyd and
Faigle showed that antimatroids characterise exactly when a greedy algorithm
solves single-processor scheduling with precedence constraints optimally.

**Citation.** E. A. Boyd and U. Faigle, "An Algorithmic Characterization of
Antimatroids," *Discrete Applied Mathematics*, vol. 28, pp. 197--205, 1990.

### 2.4  Matroid Intersection

Though single-matroid optimisation is insufficient, **matroid intersection**
(Edmonds, 1970) --- finding maximum-weight common independent sets of two
matroids --- captures several scheduling problems:

- **Bipartite matching**: assignment of operations to functional units.
- **Arborescences**: spanning structures in dependency graphs.

Weighted matroid intersection is solvable in polynomial time (Edmonds, Lawler,
Iri--Tomizawa) and may be applicable to subproblems within our scheduler,
particularly for binding (assigning operations to specific hardware units).

**Citation.** J. Edmonds, "Matroid Intersection," *Annals of Discrete
Mathematics*, vol. 4, pp. 39--49, 1979.

---

## 3  Treewidth --- Bodlaender FPT and Structured Programs

### 3.1  Treewidth of Graphs

The **treewidth** of a graph *G* measures how "tree-like" *G* is. Formally, a
tree decomposition of *G* is a tree *T* where each node (bag) contains a subset
of vertices of *G*, such that:

1. Every vertex appears in some bag.
2. For every edge *{u,v}* in *G*, some bag contains both *u* and *v*.
3. For every vertex *v*, the bags containing *v* form a connected subtree.

The **width** is (max bag size) − 1. The treewidth *tw(G)* is the minimum width
over all tree decompositions.

### 3.2  Bodlaender's FPT Algorithm

**Theorem (Bodlaender, 1996).** For any fixed constant *k*, there exists a
linear-time algorithm that determines whether *tw(G) ≤ k* and, if so, computes
an optimal tree decomposition. The running time is *O(2^{O(k³)} · n)*.

This is a **fixed-parameter tractable (FPT)** result: exponential in *k* but
linear in *n*. For small *k* this is practical.

**Citation.** H. L. Bodlaender, "A Linear-Time Algorithm for Finding
Tree-Decompositions of Small Treewidth," *SIAM Journal on Computing*, vol. 25,
no. 6, pp. 1305--1317, 1996.

### 3.3  Treewidth of Control-Flow Graphs

A landmark result establishes that **structured programs have bounded
treewidth**:

**Theorem (Thorup, 1998; refined by Gustedt, 2002).** The control-flow graph
of any goto-free C program has treewidth at most **6**.

More recently:

**Theorem (De Oliveira et al., 2023).** The control-flow graph of any
structured goto-free program has **bounded pathwidth** (a stronger property than
bounded treewidth), and a path decomposition of small width can be found in
linear time.

**Citation.** B. C. de Oliveira, D. Pereira, and F. Quintão Pereira, "The
Bounded Pathwidth of Control-Flow Graphs," *Proceedings of the ACM on
Programming Languages (OOPSLA)*, vol. 7, 2023.

### 3.4  Implications for c2v

Our restricted C has no goto, no recursion, and no function pointers. Therefore:

1. **Control-flow graphs have treewidth ≤ 6.** This is a direct consequence of
   Thorup's theorem applied to our input language.
2. **Data-flow graphs within basic blocks** are DAGs of bounded fan-in
   (determined by the ISA --- at most 3 inputs per ALU operation). The treewidth
   of such DAGs is bounded by the maximum expression nesting depth.
3. **Many NP-hard graph problems become polynomial on bounded-treewidth
   graphs**, via Courcelle's theorem (MSO₂-definable properties) or direct
   dynamic programming on tree decompositions.

**Opportunity.** If we can show that the DFGs arising from restricted C have
treewidth bounded by a small constant *k*, then scheduling (even
resource-constrained) admits an FPT algorithm with complexity
*O(f(k) · n)* where *f(k)* is exponential in *k* but *k* is small (≤ 6--10 in
practice). This would give us provably optimal schedules in linear time for
realistic inputs.

### 3.5  Thorup (1998) vs. General Java Programs

Gustedt showed treewidth ≤ 6 for structured programs, but Ranganath et al.
(2007) showed **Java programs do NOT have bounded treewidth** in general, due to
exception handling and virtual dispatch. Our restricted C avoids all such
features, so the bounded-treewidth guarantee holds.

**Citation.** V. P. Ranganath, T. Amtoft, A. Banerjee, M. B. Dwyer, and
J. Hatcliff, "The Treewidth of Java Programs," *ALENEX*, 2007.

---

## 4  Dilworth's Theorem --- Parallelism Bounds from Antichain Width

### 4.1  Statement

**Theorem (Dilworth, 1950).** In any finite partially ordered set *P*, the
minimum number of chains needed to partition *P* equals the maximum size of an
antichain in *P*.

Equivalently: **min chain cover = max antichain width**.

### 4.2  Dual: Mirsky's Theorem

**Theorem (Mirsky, 1971).** The minimum number of antichains needed to
partition *P* equals the length of the longest chain in *P*.

### 4.3  Application to Scheduling

For a DFG poset:

| Poset concept         | Scheduling interpretation              |
|-----------------------|----------------------------------------|
| Chain                 | Sequence of dependent operations       |
| Antichain             | Set of independent (parallelisable) ops|
| Max antichain width   | Maximum inherent parallelism (ILP)     |
| Longest chain length  | Critical path length (minimum latency) |
| Min chain cover size  | Minimum number of parallel "lanes"     |

**Dilworth's theorem** tells us that the **maximum parallelism** (max antichain
width *w*) equals the minimum number of sequential execution chains. This
provides a **hard lower bound** on the number of functional units needed to
achieve the minimum-latency schedule.

**Mirsky's theorem** tells us that the **critical path length** *d* (longest
chain) equals the minimum number of time-steps (antichains) needed. This is the
**ASAP lower bound** on latency.

### 4.4  Algorithmic Consequences

- **Computing max antichain**: reducible to maximum matching in a bipartite
  graph (König's theorem), solvable in *O(n^{5/2})* time via Hopcroft--Karp.
- **Computing min chain cover**: by Dilworth's theorem, equivalent to max
  antichain, and constructible via the matching.
- **Practical bound**: if our DFG has max antichain width *w = 8* and critical
  path *d = 15*, then any schedule with *L* cycles and *R* resources must
  satisfy *L ≥ d = 15* and *R ≥ w/L'* where *L'* accounts for resource
  sharing. The product *L · R ≥ n* (total work) gives the area-time lower bound.

### 4.5  Dilworth Decomposition in c2v

We should compute the Dilworth decomposition of every DFG basic block:

1. **Width *w*** gives the parallelism ceiling --- no point allocating more than
   *w* instances of any functional unit type.
2. **Chain cover** gives a natural partitioning of operations into sequential
   threads, useful for binding and register allocation.
3. **Width profile** (*w_t* = antichain width at depth *t*) gives the
   time-varying parallelism, guiding force-directed scheduling.

---

## 5  Tropical Geometry --- Critical Paths and the (max, +) Semiring

### 5.1  The Tropical Semiring

The **tropical semiring** (also called the **max-plus algebra**) is the
algebraic structure *(ℝ ∪ {-∞}, ⊕, ⊗)* where:

- *a ⊕ b = max(a, b)* (tropical addition)
- *a ⊗ b = a + b*   (tropical multiplication)

The identity for ⊕ is *-∞* and for ⊗ is *0*.

An equivalent formulation uses the **(min, +)** semiring *(ℝ ∪ {+∞}, min, +)*,
which is more natural for shortest-path problems.

### 5.2  Critical Path as Tropical Eigenvalue

Given a weighted DAG with adjacency matrix *A* (where *A_{ij}* is the latency
of the edge from *i* to *j*, or *-∞* if no edge), the **tropical matrix
power** *A^{⊗k}* gives the maximum-weight path of exactly *k* edges.

For a general (possibly cyclic) graph, the **max-plus eigenvalue** *λ* of the
matrix *A* is:

$$λ = \max_{σ \text{ circuit}} \frac{w(σ)}{|σ|}$$

the maximum average cycle weight. This is the **critical ratio** and determines
the maximum throughput of a pipelined schedule.

For a DAG (acyclic), the critical path length is simply the longest weighted
path, computable as the tropical matrix-vector product *A^{⊗(n-1)} ⊗ v₀* where
*v₀* is the vector of source-node arrival times.

### 5.3  Butkovič's Max-Linear Systems

Peter Butkovič's monograph provides the definitive treatment:

**Reference.** P. Butkovič, *Max-Linear Systems: Theory and Algorithms*,
Springer Monographs in Mathematics, 2010.

Key results relevant to scheduling:

- **Eigenvalue computation**: the max-plus eigenvalue of an *n × n* matrix can
  be found in *O(n³)* time (via Karp's algorithm for minimum mean-weight cycle
  in the max-plus dual).
- **Eigenvectors**: the max-plus eigenvector gives the steady-state schedule
  for a periodic system --- directly applicable to loop pipelining.
- **Systems of equations**: *A ⊗ x = b* in max-plus algebra corresponds to a
  system of "max-of-sums" constraints, equivalent to a set of difference
  constraints --- connecting directly to SDC scheduling.

### 5.4  Tropical Geometry of PERT Networks

Cohen et al. formalised the critical path method (CPM / PERT) in terms of
tropical geometry:

**Citation.** G. Cohen, S. Gaubert, and J.-P. Quadrat, "Duality and Separation
Theorems in Idempotent Semimodules," *Linear Algebra and its Applications*,
vol. 379, pp. 395--422, 2004.

Joswig and Loho presented the critical-path adjacency structure using tropical
geometry:

**Citation.** M. Joswig and G. Loho, "Tropical Geometry of PERT," arXiv:1202.6457, 2012.

The adjacency of critical paths --- which perturbations cause a different path
to become critical --- is described by the **tropical hypersurface** of the
network polynomial. This has applications to **sensitivity analysis**: how much
can we slow down a non-critical operation before it becomes critical?

### 5.5  Application to c2v

1. **Critical path computation** in our DFGs is already tropical matrix-vector
   multiplication. Making this explicit lets us use tropical duality for
   sensitivity analysis and slack computation.
2. **Loop pipelining** (modulo scheduling) maps to finding the max-plus
   eigenvalue of the inter-iteration dependency matrix. The **initiation
   interval (II)** lower bound is exactly this eigenvalue.
3. **Tropical convexity** may provide a framework for reasoning about the
   Pareto frontier of multi-objective scheduling (latency vs. throughput).

---

## 6  Schedule Polytopes --- Feasible Schedules as Polytope Geometry

### 6.1  The Order Polytope

Given a poset *P* on *n* elements, Stanley's **order polytope** *O(P)* is:

$$O(P) = \{ x \in [0,1]^n : x_i \leq x_j \text{ whenever } i \leq_P j \}$$

The **vertices** of *O(P)* are the characteristic vectors of antichains of *P*.
The normalised volume of *O(P)* is *|L(P)| / n!* where *|L(P)|* is the number
of linear extensions.

**Citation.** R. P. Stanley, "Two Poset Polytopes," *Discrete & Computational
Geometry*, vol. 1, pp. 9--23, 1986.

### 6.2  The SDC Polytope

In SDC-based scheduling, the feasible region is defined by a system of
**difference constraints**:

$$\sigma(v) - \sigma(u) \geq \delta(u, v) \quad \forall (u, v) \in E$$
$$0 \leq \sigma(v) \leq L \quad \forall v \in V$$

This defines a **polytope** in *ℝ^n*. The constraint matrix of a system of
difference constraints has a special structure: each inequality involves exactly
two variables with coefficients +1 and −1. The constraint matrix is a
**node-edge incidence matrix** of a directed graph.

**Key property.** Node-edge incidence matrices of directed graphs are **totally
unimodular** (TU). By the Hoffman--Kruskal theorem, any LP with a TU constraint
matrix and integer right-hand side has an integral optimal solution.

**Theorem.** The SDC polytope (without resource constraints) is an **integral
polytope**: every vertex has integer coordinates. Therefore, LP relaxation
solves the integer scheduling problem exactly.

This is why SDC scheduling works so well: the LP relaxation is tight. Adding
resource constraints destroys total unimodularity, which is why resource-
constrained scheduling is NP-hard.

**Citation.** A. Schrijver, *Theory of Linear and Integer Programming*, Wiley,
1986.

### 6.3  The Polyhedral Model (Feautrier)

For loop nests, Feautrier's approach represents iteration spaces as integer
points in Z-polytopes. A **schedule** is an affine function *Θ(S, i) = c_S · i +
d_S* mapping each iteration vector *i* of statement *S* to a time-step.

The set of all legal affine schedules forms a polytope defined by:

$$Θ(T, j) - Θ(S, i) \geq \delta \quad \forall (S, i) → (T, j) \in \text{Deps}$$

Finding the optimal schedule reduces to ILP over this polytope. Feautrier
showed how to solve this via parametric ILP (PIP), and later work by Bondhugula
et al. (the **Pluto algorithm**, PLDI 2008) provided practical heuristics.

**Citation.** P. Feautrier, "Some Efficient Solutions to the Affine Scheduling
Problem: I. One-Dimensional Time," *International Journal of Parallel
Programming*, vol. 21, no. 5, pp. 313--347, 1992.

**Citation.** U. Bondhugula, A. Hartono, J. Ramanujam, and P. Sadayappan, "A
Practical Automatic Polyhedral Parallelizer and Locality Optimizer," *PLDI*,
pp. 101--113, 2008.

### 6.4  Implications for c2v

1. **Without resource constraints**, our SDC LP always yields integer-optimal
   schedules. We get this "for free" from total unimodularity.
2. **With resource constraints**, we can use the SDC LP solution as a starting
   point and apply rounding or branch-and-bound. The gap between LP relaxation
   and integer optimum is typically small for structured DFGs.
3. **For loop pipelining**, we can extend the polyhedral model to find optimal
   II and pipeline schedules. The modulo SDC framework (Zhang, 2013) does this.

---

## 7  Rate--Distortion Analogy --- Area vs. Latency Pareto Frontier

### 7.1  The Fundamental Tradeoff

Every HLS schedule represents a point in the **(latency, area)** plane. The
**Pareto frontier** is the set of schedules where no schedule has both lower
latency and lower area. Finding the Pareto frontier is the central problem of
HLS design-space exploration (DSE).

### 7.2  Information-Theoretic Analogy

Shannon's **rate-distortion function** *R(D)* gives the minimum bit-rate
needed to represent a source with distortion ≤ *D*. The analogy to HLS:

| Rate--distortion         | HLS scheduling                        |
|--------------------------|---------------------------------------|
| Source                   | Computation (DFG)                     |
| Rate (bits/symbol)       | Area (LUTs, DSPs, FFs)                |
| Distortion               | Latency (clock cycles)                |
| R(D) curve               | Pareto frontier A(L)                  |
| Coding scheme            | Schedule + binding                    |

The Pareto frontier *A(L)* is convex (by time-sharing / resource-multiplexing
arguments) and monotonically decreasing. Its shape is determined by the
**inherent parallelism** of the DFG:

- **High parallelism** (wide antichains): steep frontier --- small latency
  reductions require large area increases.
- **Low parallelism** (narrow antichains): flat frontier --- parallelism is
  already exploited; adding area gives diminishing returns.

### 7.3  Lower Bounds

Several lower bounds constrain the Pareto frontier:

1. **Critical path bound**: *L ≥ d* (Mirsky's theorem). No schedule can be
   shorter than the longest dependency chain.
2. **Work bound**: *L · R ≥ n* where *n* is the total number of operations and
   *R* is the number of functional units. This is the "conservation of work"
   principle.
3. **Resource lower bound**: *R ≥ ⌈n / L⌉* for any target latency *L*.
4. **Antichain bound (Dilworth)**: *R ≥ w* if *L = d* (maximum parallelism
   requires at least *w* units, where *w* is the Dilworth width).

### 7.4  Design Space Exploration

HLS DSE is a multi-objective optimisation problem where a single global
optimum rarely exists. Recent approaches include:

- **Bayesian optimisation** over pragma configurations (Schafer & Wang, 2019).
- **Reinforcement learning** for directive exploration (AutoDSE, Sohrabizadeh
  et al., 2022).
- **LLM-directed** pragma insertion (Xu et al., 2025), where large language
  models suggest HLS directives.

**Citation.** S. Sohrabizadeh, C. H. Yu, M. Gao, and J. Cong, "AutoDSE:
Enabling Software Programmers to Design Efficient FPGA Accelerators," *ACM
TODAES*, vol. 27, no. 4, 2022.

**Citation.** B. Xu et al., "High-level Synthesis Directives Design
Optimization via Large Language Model," *ACM TODAES*, 2025.

### 7.5  Application to c2v

We should compute and expose the Pareto frontier to users:

1. **Sweep the latency constraint** from *d* (critical path) to *n*
   (fully sequential) and solve the resource-minimisation LP at each point.
2. **Prune dominated solutions** to extract the Pareto set.
3. **Present the tradeoff curve** so users can select their operating point
   (e.g., "minimize area with latency ≤ 20 cycles").

The area-time product *A · L* is a useful scalar metric; points on the Pareto
frontier where this product is minimised are often good default choices.

---

## 8  SDC Scheduling --- State of the Art (2006--2026)

### 8.1  Foundations: Cong & Zhang (2006)

The SDC scheduling framework was introduced by Jason Cong and Zhiru Zhang at
DAC 2006:

**Citation.** J. Cong and Z. Zhang, "An Efficient and Versatile Scheduling
Algorithm Based on SDC Formulation," *DAC*, pp. 433--438, 2006.

Core idea: encode scheduling constraints as a **system of difference
constraints** (SDC):

$$\sigma(v) - \sigma(u) \geq \delta(u,v) \quad \text{(dependency)}$$
$$\sigma(v) - \sigma(u) \leq M \quad \text{(relative timing)}$$
$$0 \leq \sigma(v) \leq L \quad \text{(latency bound)}$$

SDC constraints form a conjunction of inequalities each involving the
difference of exactly two variables. The feasibility of such a system is
equivalent to the absence of negative-weight cycles in the corresponding
**constraint graph**, testable by Bellman--Ford in *O(|V| · |E|)*.

When the objective is linear (e.g., minimise total latency, maximise slack
balance), the problem is an **LP** with a TU constraint matrix, guaranteeing
integer solutions via the simplex method or shortest-path algorithms.

### 8.2  Modulo SDC Scheduling (Zhang, 2013)

For **loop pipelining**, Zhang extended SDC to modulo scheduling:

**Citation.** Z. Zhang, "SDC-Based Modulo Scheduling for Pipeline Synthesis,"
*ICCAD*, pp. 211--218, 2013.

The key extension: for a loop with initiation interval *II*, the modulo
constraint is:

$$\sigma(v) - \sigma(u) \geq \delta(u,v) - II \cdot \text{dist}(u,v)$$

where *dist(u,v)* is the iteration distance (number of loop iterations between
the producer *u* and consumer *v*). This is still a system of difference
constraints, so the LP relaxation is tight.

Lichtenau et al. extended this with recurrence minimisation:

**Citation.** C. Lichtenau, Z. Zhang, and J. Cong, "Modulo SDC Scheduling with
Recurrence Minimization in High-Level Synthesis," *DAC*, pp. 1--6, 2014.

### 8.3  Entropy-Directed Scheduling (Shen, Cong, & Xiao, 2020)

A novel information-theoretic approach to scheduling:

**Citation.** J. Shen, J. Cong, and M. Xiao, "Entropy-Directed Scheduling for
FPGA High-Level Synthesis," *IEEE TCAD*, vol. 39, no. 12, pp. 5028--5041, 2020.

The insight: model the scheduling process as reducing **entropy** (uncertainty
about operation placement). The **maximum entropy principle** provides an
optimal assignment of operations to time-steps when no other information
distinguishes them.

Performance: EDS reduces resource usage by up to **68%** vs. force-directed
scheduling and is **294x faster**. For resource-constrained scheduling, EDS
achieves near-optimal solutions with an average speedup of **16,410x** over ILP.

### 8.4  Differentiable Scheduling (Liu et al., 2024; Cai et al., 2026)

Two breakthrough papers apply gradient-based optimisation to scheduling:

**Paper 1.** Liu et al., "Differentiable Combinatorial Scheduling at Scale,"
*ICML*, PMLR 235:31464--31476, 2024.

- Uses the **Gumbel-Softmax** trick to make discrete scheduling decisions
  differentiable.
- Introduces a **constrained Gumbel trick** to encode SDC inequalities.
- Outperforms CPLEX, Gurobi, and CP-SAT on most benchmarks.

**Paper 2.** Y. Cai, V. Bakhtazad, C. Yu, and Z. Zhang, "GauS: Differentiable
Scheduling Optimization via Gaussian Reparameterization," arXiv:2602.20427,
2026.

- Models schedules as **Gaussian distributions** over time-steps.
- Reduces the parameter space from *D · |V|* to *2|V|* (mean and variance per
  operation).
- Captures the **ordinal nature** of time (adjacent time-steps are more similar
  than distant ones), which categorical Gumbel-Softmax misses.
- Naturally supports **pipelined scheduling** via periodic Gaussian mixtures.

### 8.5  Joint Selection + Scheduling: SkyEgg (2025)

**Citation.** SkyEgg: "Joint Implementation Selection and Scheduling for
Hardware Synthesis using E-graphs," arXiv:2511.15323, 2025.

Key innovation: current HLS tools (including Vitis) **separate** implementation
selection (which hardware unit to use for each operation) from scheduling (when
to execute). SkyEgg uses **e-graphs** (equality saturation) to represent the
full space of algebraic rewrites and hardware mappings simultaneously, then
solves the joint problem as **MILP** on the saturated e-graph.

Results: **3.01x average speedup** over Vitis HLS, with improvements up to
**5.22x** for complex expressions, on Xilinx Kintex UltraScale+.

### 8.6  HACE: Tool-Agnostic CDFG Extraction (Rizzi et al., 2026)

**Citation.** C. Rizzi, S. Pfeiler, and L. Josipovic, "HACE: HLS-Tool-Agnostic
CDFG Extraction from RTL Designs," *FPGA*, 2026.

HACE reconstructs the CDFG from HLS-generated RTL, enabling fair comparison
of scheduling strategies across different HLS tools. This is relevant for
benchmarking c2v against Vitis.

### 8.7  Network-Flow SDC (Li et al., 2020)

**Citation.** Y. Li, J. Zhang, and B. Yu, "Scheduling Algorithm Based on
System of Difference Constraints Using Network Flow," *ASP-DAC*, 2020.

Reformulates SDC scheduling as a minimum-cost network flow problem, exploiting
the constraint graph structure for faster solving.

### 8.8  Summary Table: SDC Scheduling Evolution

| Year | Contribution                            | Authors              | Key advance                             |
|------|-----------------------------------------|----------------------|-----------------------------------------|
| 2006 | SDC formulation for HLS                 | Cong & Zhang         | Unified constraint framework            |
| 2013 | Modulo SDC for pipelining               | Zhang                | Loop pipelining via SDC                 |
| 2014 | Recurrence minimisation                 | Lichtenau et al.     | Better II for recurrences               |
| 2020 | Entropy-directed scheduling             | Shen, Cong, Xiao     | Information-theoretic heuristic         |
| 2020 | Network-flow SDC                        | Li, Zhang, Yu        | Faster LP via network flow              |
| 2024 | Differentiable Gumbel scheduling        | Liu et al.           | Gradient-based, beats commercial ILP    |
| 2025 | SkyEgg (e-graph joint opt)              | ---                  | Joint selection + scheduling, 3x Vitis  |
| 2026 | GauS (Gaussian reparameterization)      | Cai et al.           | Ordinal-aware differentiable scheduling |
| 2026 | HACE (CDFG extraction)                  | Rizzi et al.         | Cross-tool scheduling comparison        |

---

## 9  Vitis HLS Internals and Weaknesses

### 9.1  Architecture Overview

Vitis HLS (formerly Vivado HLS, Autopilot) is Xilinx/AMD's production C/C++ to
RTL compiler. Its scheduling pipeline is roughly:

1. **Front-end**: Clang-based C/C++ parser → LLVM IR.
2. **Optimisation passes**: standard LLVM passes + HLS-specific transforms.
3. **Scheduling**: primarily **list scheduling** with SDC constraints, guided by
   user pragmas (#pragma HLS PIPELINE, UNROLL, ARRAY_PARTITION, etc.).
4. **Binding**: greedy allocation of operations to hardware resources.
5. **RTL generation**: FSM + datapath Verilog/VHDL emission.

### 9.2  Known Weaknesses

Drawing from Cong et al., "FPGA HLS Today: Successes, Challenges, and
Opportunities," *ACM TRETS*, vol. 15, no. 4, 2022, and the SkyEgg analysis:

#### 9.2.1  Sequential Optimisation Phases

Vitis performs **implementation selection first**, then **scheduling**. Neither
phase has complete information about the other's decisions. SkyEgg showed this
leads to conservative 3-cycle solutions where joint optimisation achieves
2-cycle solutions with better resource mapping.

**Exploitable by c2v:** Our scheduler can jointly optimise selection and
scheduling, since we control both phases. Even without e-graphs, iterating
between selection and scheduling (or using a combined MILP) should beat Vitis.

#### 9.2.2  Memory Port Bottleneck

Vitis schedules memory accesses conservatively. A single dual-port BRAM
provides only **2 accesses per cycle**. If a loop body requires *R > 2* array
reads, Vitis must either:

- Increase the initiation interval (serialise), or
- Require the user to manually insert `#pragma HLS ARRAY_PARTITION`.

The user must **manually** specify array partitioning --- Vitis does not
automatically determine optimal banking.

**Exploitable by c2v:** Since we ban aliasing and have fixed-size arrays, we
can **automatically infer optimal array partitioning** via static analysis of
access patterns. Every array access is statically resolvable; we can compute
the minimum number of banks needed for conflict-free access at any target II.

#### 9.2.3  Conservative Resource Allocation

Vitis maps operations to **fabric resources** (LUTs) even when **specialised
hardware** (DSP48E2 slices) would be more efficient. This is particularly
problematic for multiply-accumulate patterns where DSP48E2 can implement the
full MAC in one slice.

**Exploitable by c2v:** We can target DSP48E2 aggressively, pattern-matching
MAC chains and fused-multiply-add trees.

#### 9.2.4  Lack of Automatic Data Caching

Vitis does not automatically cache off-chip data. Users must explicitly manage
on-chip memory (BRAM/URAM/LUT RAM) via pragmas. This is an ergonomic burden
and a source of suboptimal designs when users get it wrong.

#### 9.2.5  Memory Dependence Over-Conservatism

Without sophisticated alias analysis, Vitis assumes potential dependencies
between memory accesses to the same array, serialising operations that could
safely execute in parallel. Our no-aliasing restriction eliminates this class
of problems entirely.

#### 9.2.6  Pragma Sensitivity and Unpredictability

The UNROLL directive alone produces **unpredictable results**: greater unrolling
can yield either better or worse designs with no clear monotonic trend. This is
because unrolling interacts non-trivially with memory banking, routing
congestion, and resource limits.

**Citation.** A. Nigam et al., "Predictable Accelerator Design with
Time-Sensitive Affine Types," *PLDI*, 2020. (Demonstrates that Calyx achieves
more predictable resource allocation than Vitis.)

#### 9.2.7  Clock Frequency Challenges

Achieving high clock frequency (>300 MHz) on modern FPGAs requires awareness of
physical placement and routing congestion. Vitis HLS operates at the
behavioural level and does not model these physical effects, leading to designs
that meet timing in synthesis but fail in place-and-route.

### 9.3  Where c2v Can Win

| Vitis weakness                  | c2v advantage                                |
|---------------------------------|----------------------------------------------|
| Sequential selection/scheduling | Joint optimisation (MILP or iterative)        |
| Manual array partitioning       | Automatic: no aliasing → static analysis      |
| Conservative fabric mapping     | Aggressive DSP48E2 targeting                  |
| Pragma sensitivity              | No pragmas: compiler decides everything       |
| Alias-induced serialisation     | No aliasing in language                       |
| Unpredictable unrolling         | Principled unroll via Dilworth width analysis |
| No sensitivity analysis         | Tropical slack analysis                       |

---

## 10  Synthesis and Recommendations for c2v

### 10.1  Mathematical Tools to Implement

| Tool                          | Where in c2v               | Benefit                           |
|-------------------------------|----------------------------|-----------------------------------|
| Dilworth width computation    | Before scheduling          | Parallelism ceiling, FU bound     |
| Critical path (tropical)      | SDC initialisation         | ASAP bound, slack analysis        |
| Treewidth check               | DFG construction           | Exploit FPT if tw ≤ 6            |
| SDC LP (TU integrality)       | Core scheduler             | Exact integer optimal (no ILP!)   |
| Max-plus eigenvalue           | Loop pipeliner             | II lower bound                    |
| Pareto sweep                  | Design-space exploration   | User-facing area/latency tradeoff |
| Entropy-directed heuristic    | Resource-constrained fallback | Fast near-optimal schedules    |
| Sensitivity analysis          | Post-scheduling            | Identify slack and bottlenecks    |

### 10.2  Architectural Decisions

1. **Keep SDC as the core.** The TU integrality guarantee makes SDC the best
   foundation. Resource constraints should be handled via Lagrangian relaxation
   or iterative SDC refinement, not by switching to full ILP.

2. **Compute Dilworth decomposition eagerly.** Width and chain cover are cheap
   (*O(n^{5/2})*) and provide essential bounds that constrain the SDC LP.

3. **Exploit bounded treewidth.** Since our input is goto-free restricted C,
   treewidth is bounded. We should compute a tree decomposition and use it for
   dynamic-programming-based exact scheduling when the DFG is small enough.

4. **Implement tropical sensitivity analysis.** After scheduling, compute the
   tropical slack of every operation. This tells users (and our optimiser) which
   operations are on the critical path and where there is room for improvement.

5. **Target GauS-style differentiable scheduling** as a medium-term
   optimisation. The Gaussian reparameterization approach scales better than ILP
   and naturally handles pipelining.

6. **Joint selection and scheduling.** Even without full e-graph equality
   saturation, we can enumerate a small set of implementation choices per
   operation (e.g., LUT-based vs. DSP-based multiplication) and include the
   selection in the MILP. This is where SkyEgg's 3x over Vitis comes from.

### 10.3  Theoretical Insights for Restricted C

Our restricted C language provides mathematical structure that general HLS
cannot exploit:

- **No aliasing** → memory dependence graph is **exact** (no spurious edges).
- **No recursion** → call graph is a DAG; inlining is always possible.
- **Fixed arrays** → array sizes are compile-time constants; banking and
  partitioning are **decidable** optimisation problems.
- **No heap** → all storage is static; lifetimes are known at compile time.
- **Structured control flow** → CFG treewidth ≤ 6.

Together, these restrictions mean that our DFGs are closer to the "nice"
instances where the theoretical lower bounds (Dilworth width, critical path,
area-time product) are achievable, and the gaps exploited by papers like
SkyEgg are present in our competitors but absent in our tool.

---

## References (Consolidated)

### Lattice Theory and Linear Extensions
1. G. Brightwell and P. Winkler, "Counting Linear Extensions," *Order*, 8(3):225--242, 1991.
2. S. Dittmer and I. Pak, "Counting Linear Extensions of Restricted Posets," *Electronic J. Combinatorics*, 27(4):P4.48, 2020. arXiv:1802.06312.
3. R. P. Stanley, *Enumerative Combinatorics*, Vol. 1, Cambridge University Press, 1997.

### Matroid Theory
4. J. Edmonds, "Matroids and the Greedy Algorithm," *Mathematical Programming*, 1(1):127--136, 1971.
5. J. Edmonds, "Matroid Intersection," *Annals of Discrete Mathematics*, 4:39--49, 1979.
6. E. A. Boyd and U. Faigle, "An Algorithmic Characterization of Antimatroids," *Discrete Applied Mathematics*, 28:197--205, 1990.

### Treewidth
7. H. L. Bodlaender, "A Linear-Time Algorithm for Finding Tree-Decompositions of Small Treewidth," *SIAM J. Computing*, 25(6):1305--1317, 1996.
8. M. Thorup, "All Structured Programs Have Small Tree-Width and Good Register Allocation," *Information and Computation*, 142(2):159--181, 1998.
9. B. C. de Oliveira, D. Pereira, and F. Quintão Pereira, "The Bounded Pathwidth of Control-Flow Graphs," *OOPSLA*, 2023.
10. V. P. Ranganath et al., "The Treewidth of Java Programs," *ALENEX*, 2007.

### Order Theory
11. R. P. Dilworth, "A Decomposition Theorem for Partially Ordered Sets," *Annals of Mathematics*, 51(1):161--166, 1950.
12. L. Mirsky, "A Dual of Dilworth's Decomposition Theorem," *American Mathematical Monthly*, 78(8):876--877, 1971.

### Tropical Geometry
13. P. Butkovič, *Max-Linear Systems: Theory and Algorithms*, Springer, 2010.
14. M. Joswig and G. Loho, "Tropical Geometry of PERT," arXiv:1202.6457, 2012.
15. N. Krivulin, "Tropical Optimization Problems with Application to Project Scheduling with Minimum Makespan," arXiv:1403.0268, 2014.

### Schedule Polytopes and Polyhedral Model
16. R. P. Stanley, "Two Poset Polytopes," *Discrete & Computational Geometry*, 1:9--23, 1986.
17. A. Schrijver, *Theory of Linear and Integer Programming*, Wiley, 1986.
18. P. Feautrier, "Some Efficient Solutions to the Affine Scheduling Problem: I. One-Dimensional Time," *Int. J. Parallel Programming*, 21(5):313--347, 1992.
19. U. Bondhugula et al., "A Practical Automatic Polyhedral Parallelizer and Locality Optimizer," *PLDI*, pp. 101--113, 2008.

### SDC Scheduling
20. J. Cong and Z. Zhang, "An Efficient and Versatile Scheduling Algorithm Based on SDC Formulation," *DAC*, pp. 433--438, 2006.
21. Z. Zhang, "SDC-Based Modulo Scheduling for Pipeline Synthesis," *ICCAD*, pp. 211--218, 2013.
22. C. Lichtenau, Z. Zhang, and J. Cong, "Modulo SDC Scheduling with Recurrence Minimization in High-Level Synthesis," *DAC*, pp. 1--6, 2014.
23. J. Shen, J. Cong, and M. Xiao, "Entropy-Directed Scheduling for FPGA High-Level Synthesis," *IEEE TCAD*, 39(12):5028--5041, 2020.
24. Y. Li, J. Zhang, and B. Yu, "Scheduling Algorithm Based on SDC Using Network Flow," *ASP-DAC*, 2020.

### Differentiable and Modern Scheduling
25. Liu et al., "Differentiable Combinatorial Scheduling at Scale," *ICML*, PMLR 235:31464--31476, 2024.
26. Y. Cai et al., "GauS: Differentiable Scheduling Optimization via Gaussian Reparameterization," arXiv:2602.20427, 2026.
27. SkyEgg, "Joint Implementation Selection and Scheduling for Hardware Synthesis using E-graphs," arXiv:2511.15323, 2025.
28. C. Rizzi et al., "HACE: HLS-Tool-Agnostic CDFG Extraction from RTL Designs," *FPGA*, 2026.

### Complexity
29. M. R. Garey and D. S. Johnson, "Complexity Results for Multiprocessor Scheduling under Resource Constraints," *SIAM J. Computing*, 4(4):397--411, 1975.
30. M. R. Garey and D. S. Johnson, *Computers and Intractability: A Guide to the Theory of NP-Completeness*, Freeman, 1979.

### Vitis HLS and Design-Space Exploration
31. J. Cong et al., "FPGA HLS Today: Successes, Challenges, and Opportunities," *ACM TRETS*, 15(4), 2022.
32. A. Nigam et al., "Predictable Accelerator Design with Time-Sensitive Affine Types," *PLDI*, 2020.
33. S. Sohrabizadeh et al., "AutoDSE: Enabling Software Programmers to Design Efficient FPGA Accelerators," *ACM TODAES*, 27(4), 2022.
34. B. Xu et al., "High-level Synthesis Directives Design Optimization via Large Language Model," *ACM TODAES*, 2025.

---

*Last updated: 2026-06-25. Research conducted for the c2v project.*
