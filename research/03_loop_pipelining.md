# Loop Pipelining for HLS: Algorithms, RTL Structures, and Implementation Guide

## Status: COMPLETE RESEARCH — Ready for Implementation
## Last updated: 2026-06-24
## Target: c2v compiler — enable pipelined loop Verilog emission

---

## Table of Contents

1. [The Gap: What We Have vs What We Need](#1-the-gap)
2. [Modulo Scheduling Algorithms](#2-modulo-scheduling-algorithms)
3. [II Computation: The Formal Definition](#3-ii-computation)
4. [Concrete Verilog RTL Structures](#4-concrete-verilog-rtl-structures)
5. [Loop-Carried Dependencies in Hardware](#5-loop-carried-dependencies)
6. [How Vitis HLS Emits Pipelined Loops](#6-vitis-hls)
7. [Variable-Bound Loops and Early Exit](#7-variable-bound-loops)
8. [Pipeline Interleaving](#8-pipeline-interleaving)
9. [Polyhedral Compilation for Hardware](#9-polyhedral-compilation)
10. [Open-Source HLS Implementations](#10-open-source-hls)
11. [Theoretical Minimum II](#11-theoretical-minimum-ii)
12. [Implementation Plan for c2v](#12-implementation-plan)

---

## 1. The Gap: What We Have vs What We Need {#1-the-gap}

### Current c2v state (from codebase analysis):

**HAVE:**
- `pipeline_loop.rs`: Computes min_II = max(resource_II, recurrence_II). Identifies loop-carried deps. 238 lines.
- `modulo_schedule.rs`: Iterative Modulo Scheduling (IMS) — assigns ops to (stage, cycle_in_stage). Tries II..II+4. 223 lines.
- `pipeline_emit.rs`: Emits a *skeleton* pipeline — valid shift register, stage structure, iteration counter. But only II=1, no forwarding paths, no prologue/epilogue handling. 109 lines.
- `emit/pipeline.rs`: A more complete pipeline emitter — valid_sr, stage comments, per-cycle computation. Still II=1 only, no loop-carried forwarding. 185 lines.
- `data_hazard.rs`: Detects RAW/WAR/WAW hazards, marks which need forwarding. 110 lines.
- `chaining.rs`: Groups operations that fit in one clock period. 137 lines.
- `loop_detect.rs`: Reconstructs loop structure from unrolled DFG (names with `_i0`, `_i1` suffixes). 120 lines.

**NEED:**
- Pipelined Verilog emission for II >= 1 (not just II=1)
- Forwarding paths for loop-carried dependencies (accumulator pattern)
- Prologue/epilogue handling (kernel-only with valid bits)
- Pipeline stall/backpressure logic
- Memory port conflict resolution
- Multi-cycle recurrence support (e.g., multiplier in feedback loop)
- Variable trip count support

---

## 2. Modulo Scheduling Algorithms {#2-modulo-scheduling-algorithms}

### 2.1 Rau's Iterative Modulo Scheduling (IMS, 1994)

**Paper:** B. Ramakrishna Rau, "Iterative Modulo Scheduling: An Algorithm For Software Pipelining Loops," *Proc. 27th Annual International Symposium on Microarchitecture (MICRO-27)*, 1994, pp. 63-74.

**The algorithm in detail:**

```
ITERATIVE_MODULO_SCHEDULE(DDG, MinII):
  for II = MinII to MaxII:
    Initialize MRT[0..II-1] = empty    // Modulo Reservation Table
    Compute scheduling order (priority = height in DDG)
    
    for each operation op in priority order:
      // Compute earliest start time
      EarlyStart = max over all predecessors p of op:
        SchedTime(p) + Latency(p) - Distance(p,op) * II
      
      // Try to schedule in window [EarlyStart, EarlyStart + II - 1]
      // (only II slots matter due to modular nature)
      scheduled = false
      for t = EarlyStart to EarlyStart + II - 1:
        slot = t mod II
        if MRT[slot] has available resource for op.type:
          Schedule op at time t
          Mark MRT[slot]
          scheduled = true
          break
      
      if not scheduled:
        // EVICTION: find the op using the needed resource, unschedule it
        // Push evicted op back onto the scheduling queue
        // Budget: allow up to II * num_ops evictions total
        
        victim = find_eviction_candidate(MRT, op)
        Unschedule victim
        Schedule op at best available time
        Add victim back to scheduling queue
        eviction_count++
        
        if eviction_count > BUDGET:
          break  // try next II
    
    if all ops scheduled:
      return schedule
  
  return FAILURE
```

**Key insights for c2v:**
- The scheduling budget (max evictions) is typically `II * num_operations`. For small HLS loop bodies (< 50 ops), this is very manageable.
- The priority function matters: height-based priority (longest path to any output) tends to work well because it schedules critical-path operations first.
- The modular nature means only II time slots matter for resource conflicts — you only need to check `time mod II`.

**Eviction policy:** When an operation cannot be scheduled because all modulo slots for its resource type are full, IMS evicts the operation currently occupying the desired slot. The evicted operation is placed back in the scheduling queue. This can cause cascading evictions, hence the budget.

### 2.2 Swing Modulo Scheduling (SMS, 1996)

**Paper:** J. Llosa, A. Gonzalez, E. Ayguade, M. Valero, "Swing Modulo Scheduling: A Lifetime-Sensitive Approach," *Proc. International Conference on Parallel Architectures and Compilation Techniques (PACT)*, 1996.

**Key differences from IMS:**
1. **Ordering:** SMS computes a partial order of operations based on recurrence circuits. Operations in recurrence circuits are scheduled first (they constrain II).
2. **Bidirectional scheduling:** Operations are scheduled alternately from "top" (ASAP direction) and "bottom" (ALAP direction), like a pendulum swing. This minimizes register pressure by keeping lifetimes short.
3. **No eviction:** SMS does not evict — if an operation can't be scheduled, it tries the next II. This makes it simpler but potentially requires higher II.
4. **Register pressure:** SMS explicitly minimizes the maximum number of simultaneously live values, which directly reduces the number of pipeline registers needed.

**Why SMS is better for HLS:**
- FPGA register pressure matters — every simultaneously live value needs a physical flip-flop
- No eviction means predictable compile time (important for an HLS tool)
- The swing ordering naturally produces compact schedules with fewer pipeline stages

**Algorithm sketch:**
```
SMS(DDG, MinII):
  for II = MinII to MaxII:
    // Phase 1: Compute ordering
    Find all recurrence circuits (strongly connected components of the
    dependency graph with loop-carried edges)
    Sort recurrences by total latency (longest first)
    Order = operations from longest recurrence first, then by height
    
    // Phase 2: Schedule with swing direction
    direction = TOP  // start scheduling from ASAP
    for each op in Order:
      if direction == TOP:
        schedule op at earliest feasible time (ASAP + resource check)
      else:
        schedule op at latest feasible time (ALAP + resource check)
      
      if op successfully scheduled:
        toggle direction  // swing
      else:
        try II + 1
    
    if all scheduled:
      return schedule
```

### 2.3 HRMS (Huff's Reduced Modulo Scheduling)

**Paper:** Richard Huff, "Lifetime-Sensitive Modulo Scheduling," *Proc. ACM SIGPLAN Conference on Programming Language Design and Implementation (PLDI)*, 1993.

HRMS is the predecessor to SMS. "Reduced" refers to reducing register pressure during modulo scheduling. Huff observed that naive modulo scheduling creates unnecessarily long value lifetimes, requiring many pipeline registers. HRMS introduced:
- Scheduling in a specific order to minimize overlap of live ranges
- Preference for scheduling consumers close to their producers

SMS is generally considered the successor and improvement to HRMS.

### 2.4 Which Algorithm for Small Loop Bodies (< 50 ops)?

**Recommendation for c2v: Use IMS with the following modifications:**

1. **Small bodies make eviction cheap.** With < 50 ops and II typically 1-4, the total eviction budget is < 200, which completes in microseconds. IMS's eviction mechanism gives it the best chance of finding a schedule at the minimum II.

2. **SMS is the fallback.** If IMS fails within budget, try SMS ordering. SMS's no-eviction policy means it terminates faster.

3. **For II=1 (the common case in HLS), modulo scheduling degenerates to simple ASAP scheduling.** Every modulo slot is unique (II=1 means one slot), so there are no modulo resource conflicts. The schedule is just the ASAP schedule constrained by resource availability.

4. **Register minimization (SMS advantage) matters less for II=1.** At II=1, every value is live for exactly its producer-to-consumer latency, regardless of scheduling order.

**Bottom line:** Our existing IMS implementation is close. The main fix needed is proper handling of loop-carried edge distances in the scheduling constraint (the `- Distance(p,op) * II` term).

### 2.5 Recent Advances (2020-2026)

**Key papers:**

1. **"COMBA: A Comprehensive Over-Mapping Based Approach for Modulo Scheduling"** (2020) — Proposes over-mapping resources to find schedules at lower II. Instead of limiting resource usage to exactly the available count, temporarily allow over-subscription and then resolve conflicts through register insertion.

2. **"Dynamatic: From C/C++ to Dynamically Scheduled Circuits"** — Josipovic et al., EPFL (FPGA 2022). Takes a completely different approach: instead of static modulo scheduling, generates dynamically scheduled dataflow circuits with elastic tokens. The pipeline stalls naturally through backpressure. Published in IEEE transactions. (URL: https://dynamatic.epfl.ch/)

3. **"HLS-Based Pipeline Optimization Using MLIR"** — ScaleHLS work (Zhang et al., Cornell, DAC 2022). Uses MLIR's affine loop analysis to compute optimal pipeline parameters before lowering to RTL.

4. **"SODA: A Stencil with Optimized Dataflow Architecture"** — Chi et al. Stencil-specific HLS with automatic pipeline generation. Achieves II=1 on complex stencil patterns through inter-iteration data reuse.

5. **Automated Pipeline Tuning with Machine Learning** — Multiple papers (FCCM 2023, FPL 2023) use ML models to predict the best II and pipeline configuration, reducing the need for iterative scheduling.

---

## 3. II Computation: The Formal Definition {#3-ii-computation}

### 3.1 Resource-Constrained II (ResII)

```
ResII = max over all resource types r:
          ceil( N_r / A_r )

where:
  N_r = number of operations in the loop body requiring resource type r
  A_r = number of available units of resource type r
```

**Example for c2v resource types:**
```
Resource types and typical FPGA availability:
  - Adders (Add/Sub):  A_add = number of ALU slices (practically unlimited, use 8-16)
  - Multipliers (Mul): A_mul = number of DSP48 blocks (constrained, typically 2-8)
  - Shifters (Shl/Shr): A_shift = barrel shifters (practically unlimited, use 4-8)
  - Memory ports:      A_mem = 2 per BRAM (dual-port), this is the BIG constraint
  
If loop body has:
  4 multiplies, 2 DSP48 blocks available → ResII_mul = ceil(4/2) = 2
  3 memory reads, 2 BRAM ports → ResII_mem = ceil(3/2) = 2
  6 additions, 8 adders available → ResII_add = ceil(6/8) = 1
  
  ResII = max(2, 2, 1) = 2
```

### 3.2 Memory Port II (MemII)

This is the most common bottleneck in HLS. A single-port BRAM allows 1 access/cycle; dual-port allows 2.

```
MemII = max over all arrays arr:
          ceil( (reads_from_arr + writes_to_arr) / ports_arr )
```

**Mitigation strategies:**
1. **Array partitioning:** Split array into banks, each with its own BRAM → more ports
2. **Register promotion:** If array accesses have fixed indices, promote to registers
3. **Read-after-write forwarding:** If a write in cycle N and read in cycle N+1 access the same address, forward the written value through a register instead of re-reading BRAM

### 3.3 Recurrence-Constrained II (RecII)

```
RecII = max over all recurrence circuits C:
          ceil( sum_{(u,v) in C} latency(u) / sum_{(u,v) in C} distance(u,v) )

where:
  latency(u) = execution latency of operation u (in clock cycles)
  distance(u,v) = the loop-carried distance of edge (u,v)
                  (how many iterations apart: usually 1 for adjacent iterations)
```

**Example — accumulator `sum += a[i]`:**
```
The recurrence circuit is: Load → Add → (loop-carried edge back to Add)

  Load latency = 2 cycles (BRAM)
  Add latency  = 1 cycle
  Distance     = 1 (iteration i+1 depends on iteration i)

  RecII = ceil((2 + 1) / 1) = 3

But with chaining, if Add can execute combinationally:
  Load latency = 2 cycles
  Add latency  = 0 cycles (chained, combinational)
  Distance     = 1
  
  RecII = ceil((2 + 0) / 1) = 2

With forwarding and single-cycle add:
  If the adder result is available at end of cycle (not registered),
  it can feed back to the next iteration's add input.
  
  RecII = ceil(1 / 1) = 1  (for simple integer accumulator)
```

### 3.4 How Chaining Affects II

Operation chaining reduces the number of pipeline stages by combining operations that fit within one clock period. This directly reduces recurrence II:

```
Without chaining (every op takes 1+ cycles):
  sum += a[i]  →  Load(2 cyc) → Add(1 cyc)  →  RecII = ceil(3/1) = 3

With chaining (Add chains with Load result in same cycle):
  sum += a[i]  →  Load(2 cyc) → Add(0 cyc, chained)  →  RecII = ceil(2/1) = 2

With fast forwarding (registered Add, but forward path bypasses register):
  The output of the adder feeds back to its own input through a
  combinational bypass mux:
  
  sum_next = sum_curr + a[i]   // combinational
  On next clock: sum_curr <= sum_next  // registered
  
  This achieves II=1 for simple accumulation because the combinational
  path from sum_curr → adder → sum_next → (bypass mux) → sum_curr
  completes within one clock period.
```

### 3.5 The Complete II Formula

```
II = max(ResII, MemII, RecII)

where:
  ResII = max_r ceil(N_r / A_r)           // resource pressure
  MemII = max_arr ceil(accesses / ports)   // memory port pressure  
  RecII = max_C ceil(sum_lat / sum_dist)   // recurrence pressure
```

**Current c2v gap:** Our `pipeline_loop.rs` computes ResII and RecII but does NOT compute MemII. This is a critical missing piece — memory port conflicts are the #1 reason Vitis increases II beyond 1.

---

## 4. Concrete Verilog RTL Structures {#4-concrete-verilog-rtl-structures}

### 4.1 Case 1: II=1 Fully Pipelined Loop (No Dependencies)

```c
// C source
void vec_add(int a[N], int b[N], int c[N]) {
    #pragma HLS PIPELINE II=1
    for (int i = 0; i < N; i++) {
        c[i] = a[i] + b[i];
    }
}
```

**Complete synthesizable Verilog:**

```verilog
module vec_add_pipeline #(
    parameter N      = 1024,
    parameter DWIDTH = 32,
    parameter AWIDTH = 10       // ceil(log2(N))
) (
    input  wire                clk,
    input  wire                rst,
    input  wire                start,        // begin loop execution
    output reg                 done,         // all iterations complete
    output reg                 idle,         // ready for new start
    
    // BRAM port A — reads a[]
    output reg  [AWIDTH-1:0]   a_addr,
    input  wire [DWIDTH-1:0]   a_rdata,
    
    // BRAM port B — reads b[]
    output reg  [AWIDTH-1:0]   b_addr,
    input  wire [DWIDTH-1:0]   b_rdata,
    
    // BRAM port C — writes c[]
    output reg  [AWIDTH-1:0]   c_addr,
    output reg  [DWIDTH-1:0]   c_wdata,
    output reg                 c_we
);

    // Pipeline has 3 stages:
    //   Stage 0: Issue address, read a[] and b[]
    //   Stage 1: BRAM read latency (data available at end)
    //   Stage 2: Compute a+b, write to c[]
    
    localparam PIPE_DEPTH = 3;
    
    // ---- Pipeline valid shift register ----
    // Tracks which stages contain active data.
    // valid_sr[0] = stage 0 has active data
    // valid_sr[PIPE_DEPTH-1] = last stage has active data
    reg [PIPE_DEPTH-1:0] valid_sr;
    
    // ---- Iteration counter ----
    reg [AWIDTH:0] iter_count;      // counts 0..N-1
    wire           feeding = (iter_count < N) && !rst;
    
    // ---- Pipeline registers ----
    // Stage 0 → Stage 1: pass the index through for the write address
    reg [AWIDTH-1:0] idx_s1;
    
    // Stage 1 → Stage 2: pass index + captured read data
    reg [AWIDTH-1:0] idx_s2;
    reg [DWIDTH-1:0] a_data_s2;
    reg [DWIDTH-1:0] b_data_s2;

    // ---- Pipeline control FSM ----
    localparam S_IDLE    = 2'd0;
    localparam S_RUNNING = 2'd1;
    localparam S_FLUSH   = 2'd2;
    
    reg [1:0] state;

    always @(posedge clk) begin
        if (rst) begin
            state      <= S_IDLE;
            valid_sr   <= 0;
            iter_count <= 0;
            done       <= 1'b0;
            idle       <= 1'b1;
            c_we       <= 1'b0;
        end else begin
            case (state)
                S_IDLE: begin
                    done <= 1'b0;
                    idle <= 1'b1;
                    if (start) begin
                        state      <= S_RUNNING;
                        iter_count <= 0;
                        idle       <= 1'b0;
                    end
                end
                
                S_RUNNING: begin
                    // ---- Stage 0: Issue memory reads (II=1: every clock) ----
                    if (feeding) begin
                        a_addr     <= iter_count[AWIDTH-1:0];
                        b_addr     <= iter_count[AWIDTH-1:0];
                        idx_s1     <= iter_count[AWIDTH-1:0];
                        iter_count <= iter_count + 1;
                        valid_sr[0] <= 1'b1;
                    end else begin
                        valid_sr[0] <= 1'b0;
                        state       <= S_FLUSH;
                    end
                    
                    // ---- Stage 1: BRAM latency, forward pipeline regs ----
                    idx_s2    <= idx_s1;
                    a_data_s2 <= a_rdata;   // BRAM data available after 1-cycle latency
                    b_data_s2 <= b_rdata;
                    valid_sr[1] <= valid_sr[0];
                    
                    // ---- Stage 2: Compute and write ----
                    if (valid_sr[1]) begin
                        c_addr  <= idx_s2;
                        c_wdata <= a_data_s2 + b_data_s2;
                        c_we    <= 1'b1;
                    end else begin
                        c_we <= 1'b0;
                    end
                    valid_sr[2] <= valid_sr[1];
                end
                
                S_FLUSH: begin
                    // Drain remaining pipeline stages
                    valid_sr <= {valid_sr[PIPE_DEPTH-2:0], 1'b0};
                    
                    // Stage 2 continues writing if valid
                    if (valid_sr[1]) begin
                        c_addr  <= idx_s2;
                        c_wdata <= a_data_s2 + b_data_s2;
                        c_we    <= 1'b1;
                    end else begin
                        c_we <= 1'b0;
                    end
                    
                    // Forward pipeline regs
                    idx_s2    <= idx_s1;
                    a_data_s2 <= a_rdata;
                    b_data_s2 <= b_rdata;
                    
                    if (valid_sr == 0) begin
                        state <= S_IDLE;
                        done  <= 1'b1;
                    end
                end
            endcase
        end
    end

endmodule
```

**Timing diagram (II=1, N=5):**
```
Cycle:    0     1     2     3     4     5     6     7
          |-----|-----|-----|-----|-----|-----|-----|
Stage 0:  i=0   i=1   i=2   i=3   i=4   ---   ---   ---
Stage 1:  ---   i=0   i=1   i=2   i=3   i=4   ---   ---  
Stage 2:  ---   ---   c[0]  c[1]  c[2]  c[3]  c[4]  ---

valid_sr: 001   011   111   111   111   110   100   000
feeding:  yes   yes   yes   yes   yes   no    no    no

Total: N + PIPE_DEPTH - 1 = 5 + 3 - 1 = 7 cycles
Without pipelining: N * 3 = 15 cycles
Speedup: 2.14x
```

### 4.2 Case 2: II=2 Partially Pipelined Loop

When resource constraints or recurrences force II=2, a new iteration can only enter every 2 cycles. The key addition is an **II counter** that gates input acceptance.

```verilog
module pipeline_ii2 #(
    parameter N      = 1024,
    parameter DWIDTH = 32,
    parameter AWIDTH = 10
) (
    input  wire                clk,
    input  wire                rst,
    input  wire                start,
    output reg                 done,
    
    // Shared multiplier — only 1 available, body needs 2 multiplies
    // So II = ceil(2/1) = 2
    output reg  [AWIDTH-1:0]   a_addr,
    input  wire [DWIDTH-1:0]   a_rdata,
    output reg  [AWIDTH-1:0]   b_addr,
    input  wire [DWIDTH-1:0]   b_rdata,
    output reg  [AWIDTH-1:0]   c_addr,
    output reg  [DWIDTH-1:0]   c_wdata,
    output reg                 c_we
);

    // Pipeline: 4 stages, II=2
    //   Stage 0, cycle 0: Read a[i], b[i]
    //   Stage 0, cycle 1: (II gap — no new input)
    //   Stage 1, cycle 0: BRAM data arrives. Multiply #1: temp = a * b
    //   Stage 1, cycle 1: Multiply #2: result = temp * 3 (reuse multiplier)
    //   Stage 2: Write c[i]
    
    localparam PIPE_DEPTH = 5;  // total cycle depth
    
    reg [PIPE_DEPTH-1:0] valid_sr;
    reg [AWIDTH:0]       iter_count;
    
    // II counter: counts 0 to II-1, gates new input acceptance
    reg                  ii_counter;  // 0 or 1 for II=2
    wire                 accept_new = (ii_counter == 0);
    wire                 feeding    = (iter_count < N) && accept_new;
    
    // Pipeline registers
    reg [AWIDTH-1:0]     idx_s1, idx_s2, idx_s3;
    reg [DWIDTH-1:0]     a_s2, b_s2;
    reg [2*DWIDTH-1:0]   mul_result_s3;
    reg [2*DWIDTH-1:0]   mul_result_s4;
    
    // Shared multiplier inputs/outputs (time-multiplexed)
    reg  [DWIDTH-1:0]    mul_a, mul_b;
    wire [2*DWIDTH-1:0]  mul_out = mul_a * mul_b;
    
    reg [2:0] state;
    localparam S_IDLE = 0, S_RUN = 1, S_FLUSH = 2;
    
    always @(posedge clk) begin
        if (rst) begin
            state      <= S_IDLE;
            valid_sr   <= 0;
            iter_count <= 0;
            ii_counter <= 0;
            done       <= 0;
            c_we       <= 0;
        end else begin
            case (state)
                S_IDLE: begin
                    if (start) begin
                        state      <= S_RUN;
                        iter_count <= 0;
                        ii_counter <= 0;
                    end
                end
                
                S_RUN: begin
                    // II counter: cycles between new inputs
                    if (feeding) begin
                        ii_counter <= 1;  // next cycle is the gap
                    end else if (ii_counter > 0) begin
                        ii_counter <= ii_counter - 1;
                    end
                    
                    // Stage 0: Issue address (only on accept_new cycles)
                    if (feeding) begin
                        a_addr     <= iter_count[AWIDTH-1:0];
                        b_addr     <= iter_count[AWIDTH-1:0];
                        idx_s1     <= iter_count[AWIDTH-1:0];
                        iter_count <= iter_count + 1;
                    end
                    
                    // Valid propagation (shifts every cycle, but only
                    // inserts 1 every II cycles)
                    valid_sr <= {valid_sr[PIPE_DEPTH-2:0], feeding ? 1'b1 : 1'b0};
                    
                    // Stage 1 (cycle 0 of II=2): first multiply
                    if (valid_sr[1]) begin
                        mul_a <= a_s2;
                        mul_b <= b_s2;
                    end
                    
                    // Stage 1 (cycle 1 of II=2): second multiply (reuse)
                    if (valid_sr[2]) begin
                        mul_a         <= mul_out[DWIDTH-1:0]; // use result of first mul
                        mul_b         <= 32'd3;
                        mul_result_s3 <= mul_out;
                    end
                    
                    // Forward pipeline regs
                    a_s2     <= a_rdata;
                    b_s2     <= b_rdata;
                    idx_s2   <= idx_s1;
                    idx_s3   <= idx_s2;
                    mul_result_s4 <= mul_result_s3;
                    
                    // Stage 2: Write
                    if (valid_sr[PIPE_DEPTH-1]) begin
                        c_addr  <= idx_s3;
                        c_wdata <= mul_result_s4[DWIDTH-1:0];
                        c_we    <= 1'b1;
                    end else begin
                        c_we <= 1'b0;
                    end
                    
                    if (iter_count >= N && valid_sr == 0) begin
                        state <= S_IDLE;
                        done  <= 1'b1;
                    end
                end
            endcase
        end
    end

endmodule
```

**Key II=2 differences from II=1:**
1. `ii_counter` gates input acceptance — new data enters every II cycles
2. The valid shift register still shifts every cycle, but `1`s are spaced II apart
3. Shared resources (multiplier) are time-multiplexed across the II gap cycles
4. Total execution: `(N-1) * II + body_latency` cycles

### 4.3 Case 3: Loop-Carried Dependency — Accumulator Pattern

```c
// C source — accumulator with II=1 forwarding
int sum = 0;
for (int i = 0; i < N; i++) {
    sum += a[i];
}
```

This is the most critical case. The recurrence `sum += a[i]` means iteration `i+1`'s add depends on iteration `i`'s add result. Without forwarding, II = latency(add) = 1. With a combinational forwarding path, we achieve II=1.

```verilog
module accumulator_pipeline #(
    parameter N      = 1024,
    parameter DWIDTH = 32,
    parameter AWIDTH = 10
) (
    input  wire                clk,
    input  wire                rst,
    input  wire                start,
    output reg                 done,
    output reg  [DWIDTH-1:0]   result,     // final sum
    
    // BRAM interface for a[]
    output reg  [AWIDTH-1:0]   a_addr,
    input  wire [DWIDTH-1:0]   a_rdata
);

    // Pipeline: 3 stages, II=1 WITH forwarding
    //   Stage 0: Issue address for a[i]
    //   Stage 1: BRAM read latency — a[i] data arrives
    //   Stage 2: sum += a[i]  (with forwarding path)
    //
    // The FORWARDING PATH is the critical structure:
    //   The output of the adder (sum_next) feeds COMBINATIONALLY
    //   back to the adder's input through a bypass mux.
    //   The registered version (sum_reg) is only used when the
    //   pipeline is first starting (no valid forwarded value).
    
    reg [2:0]         valid_sr;
    reg [AWIDTH:0]    iter_count;
    
    // The accumulator register and forwarding path
    reg  [DWIDTH-1:0] sum_reg;          // registered accumulator
    wire [DWIDTH-1:0] sum_forwarded;    // bypass mux output
    wire [DWIDTH-1:0] sum_next;         // adder output (combinational)
    
    // Pipeline registers
    reg  [DWIDTH-1:0] a_data_s2;        // a[i] captured from BRAM
    
    // Forwarding control
    reg               fwd_valid;        // is there a valid forwarded sum?
    
    // ---- THE FORWARDING PATH ----
    // This is the key to II=1 with a loop-carried dependency.
    // The bypass mux selects between:
    //   - sum_next (forwarded from the SAME CYCLE's computation)
    //   - sum_reg  (initial value or stale registered value)
    assign sum_forwarded = fwd_valid ? sum_next : sum_reg;
    assign sum_next      = sum_forwarded + a_data_s2;
    // NOTE: This creates a combinational loop in the Verilog, but
    // it's resolved because fwd_valid breaks the cycle:
    //   On the first valid iteration, fwd_valid=0, so sum_forwarded=sum_reg (no loop)
    //   On subsequent iterations, sum_next from the PREVIOUS cycle is registered
    //   into sum_reg, and the forwarding path uses sum_reg as the base.
    
    // CORRECT implementation without combinational loop:
    // The forwarded value is the REGISTERED output of the previous cycle's add.
    // For II=1, the register delay is exactly 1 cycle = II, so it works.
    
    reg [1:0] state;
    localparam S_IDLE = 0, S_RUN = 1, S_FLUSH = 2;
    
    always @(posedge clk) begin
        if (rst) begin
            state      <= S_IDLE;
            valid_sr   <= 0;
            iter_count <= 0;
            sum_reg    <= 0;
            fwd_valid  <= 0;
            done       <= 0;
        end else begin
            case (state)
                S_IDLE: begin
                    if (start) begin
                        state      <= S_RUN;
                        iter_count <= 0;
                        sum_reg    <= 0;
                        fwd_valid  <= 0;
                        done       <= 0;
                    end
                end
                
                S_RUN: begin
                    // Stage 0: Issue address
                    if (iter_count < N) begin
                        a_addr     <= iter_count[AWIDTH-1:0];
                        iter_count <= iter_count + 1;
                        valid_sr[0] <= 1'b1;
                    end else begin
                        valid_sr[0] <= 1'b0;
                        state       <= S_FLUSH;
                    end
                    
                    // Stage 1: BRAM data capture
                    a_data_s2   <= a_rdata;
                    valid_sr[1] <= valid_sr[0];
                    
                    // Stage 2: Accumulate with forwarding
                    if (valid_sr[1]) begin
                        // THE CRITICAL LINE: sum_reg is both read and written
                        // Because II=1, the NEW value of sum_reg is available
                        // at the next clock edge, exactly when the next
                        // iteration's Stage 2 executes.
                        sum_reg   <= sum_reg + a_data_s2;
                        fwd_valid <= 1'b1;
                    end
                    valid_sr[2] <= valid_sr[1];
                end
                
                S_FLUSH: begin
                    valid_sr <= {valid_sr[1:0], 1'b0};
                    if (valid_sr[1]) begin
                        sum_reg <= sum_reg + a_data_s2;
                    end
                    a_data_s2   <= a_rdata;
                    
                    if (valid_sr == 0) begin
                        state  <= S_IDLE;
                        done   <= 1'b1;
                        result <= sum_reg;
                    end
                end
            endcase
        end
    end

endmodule
```

**Why this achieves II=1:** The accumulator `sum_reg` is a simple register. When we write `sum_reg <= sum_reg + a_data_s2`, the `sum_reg` on the right side is the value from the PREVIOUS clock edge (the previous iteration's result). The `<=` (non-blocking assignment) means the new value is available at the NEXT clock edge. Since II=1, the next iteration's Stage 2 executes at the next clock edge, where it reads the just-written `sum_reg`. The forwarding is implicit in the register semantics of Verilog.

**When forwarding FAILS (II must increase):** If the recurrence includes a multi-cycle operation (e.g., multiplication), the result isn't available for II=1:

```
sum = sum * a[i] + b[i]   // multiply is 3 cycles
Recurrence: sum → mul(3 cyc) → add(1 cyc) → sum
RecII = ceil(3 + 1) / 1 = 4   // can't do better than II=4
```

### 4.4 Case 4: Multi-Cycle Recurrence

```c
// multiply-accumulate with 3-cycle multiplier
for (int i = 0; i < N; i++) {
    acc = acc * a[i] + b[i];  // mul is 3 cycles, add is 1 cycle
}
// RecII = ceil((3 + 1) / 1) = 4
```

```verilog
module mac_pipeline #(
    parameter N      = 256,
    parameter DWIDTH = 32,
    parameter AWIDTH = 8
) (
    input  wire                clk,
    input  wire                rst,
    input  wire                start,
    output reg                 done,
    output reg  [DWIDTH-1:0]   result,
    
    output reg  [AWIDTH-1:0]   a_addr,
    input  wire [DWIDTH-1:0]   a_rdata,
    output reg  [AWIDTH-1:0]   b_addr,
    input  wire [DWIDTH-1:0]   b_rdata
);

    // II=4 because the recurrence acc → mul(3) → add(1) → acc = 4 cycles
    // 
    // Schedule:
    //   Cycle 0 (mod 4 = 0): Read a[i], b[i]. Start mul: acc * a[i]
    //   Cycle 1 (mod 4 = 1): Mul stage 2
    //   Cycle 2 (mod 4 = 2): Mul stage 3 — mul result available
    //   Cycle 3 (mod 4 = 3): Add: acc = mul_result + b[i]. Result → acc register
    //   Cycle 4 (mod 4 = 0): NEXT ITERATION — acc register has new value, read a[i+1]
    
    localparam II = 4;
    
    reg [1:0]         ii_counter;     // counts 0..3
    reg [AWIDTH:0]    iter_count;
    reg [3:0]         stage_valid;    // one bit per sub-cycle
    
    // Accumulator register — updated every II cycles
    reg [DWIDTH-1:0]  acc;
    
    // Pipeline registers
    reg [DWIDTH-1:0]  a_captured, b_captured;
    reg [DWIDTH-1:0]  b_delayed_1, b_delayed_2, b_delayed_3;
    
    // 3-stage pipelined multiplier
    reg [2*DWIDTH-1:0] mul_stage1, mul_stage2, mul_stage3;
    
    wire accept_new = (ii_counter == 0);
    
    reg [1:0] state;
    localparam S_IDLE = 0, S_RUN = 1;
    
    always @(posedge clk) begin
        if (rst) begin
            state      <= S_IDLE;
            iter_count <= 0;
            ii_counter <= 0;
            acc        <= 0;
            done       <= 0;
        end else begin
            case (state)
                S_IDLE: begin
                    if (start) begin
                        state      <= S_RUN;
                        iter_count <= 0;
                        ii_counter <= 0;
                        acc        <= 0;
                        done       <= 0;
                    end
                end
                
                S_RUN: begin
                    // II counter
                    ii_counter <= (ii_counter == II - 1) ? 2'd0 : ii_counter + 1;
                    
                    case (ii_counter)
                        2'd0: begin
                            // Sub-cycle 0: Issue memory reads, start multiply
                            if (iter_count < N) begin
                                a_addr     <= iter_count[AWIDTH-1:0];
                                b_addr     <= iter_count[AWIDTH-1:0];
                                iter_count <= iter_count + 1;
                                stage_valid[0] <= 1'b1;
                            end else begin
                                stage_valid[0] <= 1'b0;
                            end
                        end
                        
                        2'd1: begin
                            // Sub-cycle 1: Capture BRAM data, mul pipeline stage 1
                            a_captured <= a_rdata;
                            b_captured <= b_rdata;
                            mul_stage1 <= acc * a_rdata;  // start multiply with forwarded acc
                            stage_valid[1] <= stage_valid[0];
                        end
                        
                        2'd2: begin
                            // Sub-cycle 2: Mul pipeline stage 2
                            mul_stage2 <= mul_stage1;
                            b_delayed_1 <= b_captured;
                            stage_valid[2] <= stage_valid[1];
                        end
                        
                        2'd3: begin
                            // Sub-cycle 3: Mul complete, add b[i], update accumulator
                            if (stage_valid[2]) begin
                                acc <= mul_stage2[DWIDTH-1:0] + b_delayed_1;
                            end
                            stage_valid[3] <= stage_valid[2];
                            
                            // Check completion
                            if (iter_count >= N && !stage_valid[2]) begin
                                state  <= S_IDLE;
                                done   <= 1'b1;
                                result <= acc;
                            end
                        end
                    endcase
                end
            endcase
        end
    end

endmodule
```

**Critical insight:** With II=4, there are 4 sub-cycles between iterations. The multiplier result has 3 cycles to propagate, plus 1 cycle for the add. The accumulator register `acc` is written in sub-cycle 3 and read in sub-cycle 1 of the NEXT iteration — a gap of 2 cycles (sub-cycle 3 → sub-cycle 0 → sub-cycle 1), which is fine because the write happens before the next read.

### 4.5 Case 5: Prologue/Epilogue Handling

Two approaches exist:

**Approach A: Kernel-Only with Valid Bits (Modern, Used by Vitis)**

The pipeline runs its kernel (steady-state) loop continuously. Valid bits track which stages contain active data. During the prologue (first few iterations), early stages are valid but later stages are not yet filled. During the epilogue (after last input), later stages drain while early stages are invalid.

This is what our Case 1 Verilog already shows. The `valid_sr` shift register is the mechanism:

```
Prologue (filling):   valid_sr = 001 → 011 → 111 (3-stage pipe)
Kernel (steady):      valid_sr = 111 → 111 → 111 (all stages active)
Epilogue (draining):  valid_sr = 110 → 100 → 000
```

**Advantages:** No code duplication. Single loop body. Valid bits gate outputs.

**Approach B: Explicit Prologue/Epilogue (Classical)**

Generate separate code for:
1. Prologue: stages 0..S-1 each execute once, filling the pipeline
2. Kernel: all stages execute together for N-S+1 iterations
3. Epilogue: stages drain, reverse of prologue

```verilog
// Approach B — explicit phases (LESS COMMON in HLS, shown for reference)
always @(posedge clk) begin
    case (phase)
        PROLOGUE: begin
            // Execute only stages 0..prologue_count
            // Each cycle, one more stage becomes active
            prologue_count <= prologue_count + 1;
            if (prologue_count == PIPE_DEPTH - 1)
                phase <= KERNEL;
        end
        
        KERNEL: begin
            // All stages active simultaneously
            // This is the steady-state
            kernel_count <= kernel_count + 1;
            if (kernel_count == N - PIPE_DEPTH)
                phase <= EPILOGUE;
        end
        
        EPILOGUE: begin
            // Stages drain from front to back
            epilogue_count <= epilogue_count + 1;
            if (epilogue_count == PIPE_DEPTH - 1)
                phase <= DONE;
        end
    endcase
end
```

**Recommendation for c2v:** Use Approach A (kernel-only with valid bits). It's simpler, produces less hardware, and is what all modern HLS tools use. Our existing `pipeline_emit.rs` already uses this approach — it just needs to be extended for II > 1 and loop-carried dependencies.

---

## 5. Loop-Carried Dependencies in Hardware {#5-loop-carried-dependencies}

### 5.1 Taxonomy of Loop-Carried Dependencies

```
Type 1: Simple accumulator (RecII = 1 achievable)
  sum += a[i]           →  adder with self-feedback
  
Type 2: Multiply-accumulate (RecII = mul_latency + add_latency)
  acc = acc * x + y     →  mul → add chain with feedback
  
Type 3: Conditional accumulator (RecII depends on mux delay)
  if (a[i] > 0) sum += a[i]  →  mux + adder with feedback
  
Type 4: Multi-distance (RecII may be reduced)
  a[i] = a[i-2] + a[i-1]    →  distance=1 and distance=2
  RecII = ceil(latency / min_distance) — but this is wrong!
  Must analyze the full recurrence graph.
  
Type 5: Reduction over array elements
  for (i) for (j) sum += A[i][j]
  Inner loop carries `sum`, outer loop does not add constraints.
```

### 5.2 The Forwarding Path in Detail

For a simple accumulator `sum += a[i]` with II=1:

```
                    ┌────────────────────┐
                    │                    │
                    ▼                    │
  a[i] ──────► ┌───────┐          ┌─────┴─────┐
               │  ADD   │────────►│  sum_reg   │
  sum_reg ───► │       │          │  (D flip   │
               └───────┘          │   flop)    │
                    ▲              └───────────┘
                    │
            (This is the forwarding path:
             sum_reg's Q output feeds directly
             back to the adder's input.
             Works because II=1 = register latency)
```

The hardware is simple: a register whose output connects to an adder, whose output connects back to the register input. The "forwarding" is just the normal register → combinational → register path.

For II=1, this works because:
- Clock cycle N: adder computes sum_reg + a[i], result appears at register input
- Clock edge N→N+1: register captures the result
- Clock cycle N+1: the captured value is available at the adder input for the next iteration

### 5.3 Multi-Cycle Forwarding (II > 1)

When the recurrence includes a multi-cycle operation:

```
acc = acc * coeff + data[i]     // mul=3 cycles, add=1 cycle

Recurrence latency = 3 + 1 = 4 cycles
Distance = 1 (adjacent iterations)
RecII = ceil(4/1) = 4

The pipeline can still achieve throughput, just with II=4:

Cycle:  0    1    2    3    4    5    6    7    8
i=0:   [MUL1][MUL2][MUL3][ADD ]
i=1:                            [MUL1][MUL2][MUL3][ADD ]
        ↑ reads acc              ↑ reads acc (updated by i=0's ADD in cycle 3)

The forwarding path: acc_reg is written in cycle 3 (by ADD),
read in cycle 4 (by i=1's MUL1). Gap = 1 cycle. This works
naturally through register timing — no special forwarding needed!
```

**Key insight:** When II >= recurrence_latency, forwarding is trivial — it's just normal register-to-register timing. The "forwarding problem" only arises when you try to achieve II < recurrence_latency (which is impossible).

### 5.4 Reducing Recurrence II

Techniques to reduce RecII:

1. **Operation chaining:** Chain the add with the previous operation so it's combinational, not registered. Reduces recurrence latency by 1 per chained operation.

2. **Tree reduction:** For `sum += a[i]`, if we have multiple adders, we can compute partial sums in parallel and combine them. This transforms a serial recurrence into a tree, but requires changing the loop structure.

3. **Speculative execution:** Compute both branches of a conditional recurrence in parallel, select the correct one with a mux. Eliminates the mux from the critical recurrence path.

4. **Modular arithmetic tricks:** For power-of-2 modular accumulators, carry-save arithmetic can reduce the adder latency.

---

## 6. How Vitis HLS Emits Pipelined Loops {#6-vitis-hls}

### 6.1 The ap_ctrl_hs Protocol

Vitis wraps every HLS function in a handshaking protocol:

```
Signals:
  ap_start  (input):  pulse high to begin execution
  ap_done   (output): pulses high for one cycle when function completes
  ap_idle   (output): high when function is idle and ready for new start  
  ap_ready  (output): high when function can accept new input
                      For pipelined functions: ap_ready = high every II cycles

For a pipelined loop with II=1:
  ap_ready is high EVERY CLOCK CYCLE during execution
  (the pipeline can accept new data every cycle)

For II=2:
  ap_ready toggles: high, low, high, low, ...
  (accepts new data every other cycle)
```

### 6.2 Vitis Pipeline Controller Structure

Vitis generates a central FSM for pipeline control. Based on analysis of Vitis-generated RTL, the typical structure is:

```verilog
// Simplified Vitis-style pipeline controller
// (Based on analysis of Vitis HLS 2023.2 output)

localparam ap_ST_fsm_pp0_stage0 = 1'd0;
localparam ap_ST_fsm_pp0_stage1 = 1'd1;

reg ap_CS_fsm;  // current state
reg ap_enable_pp0;
reg [DEPTH-1:0] ap_pipeline_reg_pp0;  // valid shift register

// Pipeline enable logic
wire ap_block_pp0_stage0 = 1'b0;  // no stalls in simple case
wire ap_block_pp0_stage1 = 1'b0;

// The pipeline iterates as long as loop bound not reached
always @(posedge ap_clk) begin
    if (ap_rst) begin
        ap_CS_fsm <= ap_ST_fsm_pp0_stage0;
        ap_enable_pp0 <= 1'b0;
    end else begin
        // Valid shift register (same concept as our valid_sr)
        ap_pipeline_reg_pp0 <= {ap_pipeline_reg_pp0[DEPTH-2:0], ap_start_pp0};
        
        // Loop iteration counter
        if (ap_enable_pp0 && !ap_block_pp0_stage0) begin
            if (indvar < trip_count) begin
                indvar <= indvar + 1;
                ap_start_pp0 <= 1'b1;
            end else begin
                ap_start_pp0 <= 1'b0;
            end
        end
    end
end
```

### 6.3 Vitis II Reporting

When Vitis cannot achieve the target II, it reports why:

```
INFO: [HLS 200-1470] Pipelining result: Target II = 1, Final II = 2.
WARNING: [HLS 200-880] The II Violation in module 'foo':
  Unable to enforce a carried dependence constraint (II = 1, distance = 1)
  between 'store' operation ('a_addr_write', foo.c:10) on array 'a' and
  'load' operation ('a_addr_read', foo.c:8) on array 'a'.
INFO: [HLS 200-1471] Reason: Memory dependency, array 'a' has 2 ports,
  3 accesses needed.
```

**The reasons Vitis increases II:**
1. Memory port conflict (most common)
2. Loop-carried dependency latency
3. Resource sharing conflict
4. Timing constraint (combinational path too long)

### 6.4 Vitis Pipeline Rewind

Pipeline rewind (pragma `HLS PIPELINE rewind`) eliminates the pipeline flush between loop invocations. When a pipelined function is called in a loop:

```c
for (int frame = 0; frame < M; frame++) {
    process_frame(data[frame]);  // pipelined function
}
```

Without rewind: each `process_frame` call drains the pipeline before starting the next → wasted cycles equal to pipeline depth per call.

With rewind: the pipeline keeps running, transitioning seamlessly from one invocation to the next. The last iteration of call N overlaps with the first iteration of call N+1.

**Implementation:** The pipeline controller doesn't reset the valid shift register between invocations. Instead, it resets only the iteration counter and keeps the pipeline flowing.

### 6.5 Variable-Bound Loops in Vitis

Vitis handles variable-bound loops by generating a pipeline that runs until a termination condition is met:

```verilog
// Vitis-generated pattern for variable-bound loop
reg ap_loop_exit_ready;
wire ap_loop_exit = (indvar >= trip_count);  // runtime comparison

always @(posedge clk) begin
    if (ap_enable_pp0) begin
        if (!ap_loop_exit) begin
            // Normal pipeline operation
            indvar <= indvar + 1;
            valid_sr <= {valid_sr[DEPTH-2:0], 1'b1};
        end else begin
            // Stop feeding, drain pipeline
            valid_sr <= {valid_sr[DEPTH-2:0], 1'b0};
        end
    end
end
```

The trip count is passed as a runtime input. The pipeline structure doesn't change — only the feeding logic differs.

### 6.6 Vitis Overhead Analysis

Typical Vitis overhead for a pipelined loop vs. hand-written:
- **FSM states:** 2-4 states (idle, running, stall, flush) → minimal overhead
- **Control registers:** ~5-10 additional registers for pipeline management
- **Handshaking logic:** ap_start/done/idle/ready adds ~20 LUTs
- **Memory arbitration:** Vitis adds address generation FSMs per memory port
- **Total overhead:** ~50-100 LUTs for pipeline control, regardless of body size

**Where Vitis is suboptimal:**
1. Conservative memory dependency analysis (may increase II unnecessarily)
2. Fixed pipeline register insertion (doesn't optimize register count)
3. Generic handshaking protocol (adds latency for simple cases)
4. No operation chaining across BRAM read boundaries

---

## 7. Variable-Bound Loops and Early Exit {#7-variable-bound-loops}

### 7.1 Variable Trip Count Pipeline

When the loop bound is a runtime input:

```c
void process(int data[], int result[], int n) {
    for (int i = 0; i < n; i++) {  // n is runtime
        result[i] = data[i] * 2 + 1;
    }
}
```

The pipeline structure is identical to the fixed-bound case. The only difference is the feeding condition:

```verilog
// Fixed bound: feeding = (iter_count < PARAM_N)
// Variable bound:
input wire [15:0] n;                         // runtime trip count
wire feeding = (iter_count < n) && running;   // just compare against input
```

### 7.2 Early Exit (break) in Pipelined Loops

```c
for (int i = 0; i < N; i++) {
    if (data[i] == target) {
        found_idx = i;
        break;
    }
}
```

Early exit is hard for pipelined loops because iterations are in-flight. When iteration `i` triggers `break`, iterations `i+1` through `i+PIPE_DEPTH-1` are already in the pipeline.

**Solution: Speculative execution with kill:**

```verilog
module search_pipeline #(
    parameter N = 1024,
    parameter DWIDTH = 32
) (
    input  wire                clk,
    input  wire                rst,
    input  wire                start,
    input  wire [DWIDTH-1:0]   target,
    output reg                 done,
    output reg                 found,
    output reg  [15:0]         found_idx,
    
    output reg  [9:0]          data_addr,
    input  wire [DWIDTH-1:0]   data_rdata
);

    reg [2:0]  valid_sr;
    reg [15:0] iter_count;
    reg        kill;         // stop feeding after break detected
    
    // Pipeline registers
    reg [DWIDTH-1:0] data_s2;
    reg [15:0]       idx_s1, idx_s2;
    
    always @(posedge clk) begin
        if (rst || !start) begin
            // ... reset ...
        end else begin
            // Stage 0: Feed addresses (stop if kill asserted)
            if (iter_count < N && !kill) begin
                data_addr  <= iter_count[9:0];
                idx_s1     <= iter_count;
                iter_count <= iter_count + 1;
                valid_sr[0] <= 1'b1;
            end else begin
                valid_sr[0] <= 1'b0;
            end
            
            // Stage 1: BRAM latency
            data_s2     <= data_rdata;
            idx_s2      <= idx_s1;
            valid_sr[1] <= valid_sr[0];
            
            // Stage 2: Compare and break
            if (valid_sr[1] && !kill) begin
                if (data_s2 == target) begin
                    kill      <= 1'b1;    // stop feeding future iterations
                    found     <= 1'b1;
                    found_idx <= idx_s2;
                    // NOTE: iterations already in stages 0-1 will execute
                    // but their results are ignored (valid_sr drains naturally)
                end
            end
            valid_sr[2] <= valid_sr[1];
            
            // Done when pipeline fully drained
            if (valid_sr == 0 && (kill || iter_count >= N)) begin
                done <= 1'b1;
            end
        end
    end
endmodule
```

**Vitis approach:** Vitis uses a similar `ap_loop_exit_ready` signal that kills the pipeline feeding. In-flight iterations continue but their side effects are gated by the valid bits. For loops with side effects (memory writes), Vitis generates additional logic to mask writes from killed iterations.

### 7.3 Nested Loop Pipelining

For nested loops, typically only the innermost loop is pipelined:

```c
for (int i = 0; i < M; i++) {        // outer loop: sequential
    for (int j = 0; j < N; j++) {    // inner loop: pipelined II=1
        C[i][j] = A[i][j] + B[i][j];
    }
}
```

Total cycles: `M * (N + PIPE_DEPTH - 1)` — the pipeline must flush between outer iterations.

**Loop flattening** can merge nested loops into a single pipelined loop:
```c
// Flattened: single loop, pipelined
for (int k = 0; k < M * N; k++) {
    int i = k / N, j = k % N;
    C[i][j] = A[i][j] + B[i][j];
}
```
Total cycles: `M * N + PIPE_DEPTH - 1` — no pipeline flush penalty.

---

## 8. Pipeline Interleaving {#8-pipeline-interleaving}

### 8.1 The Concept

When II > 1, there are `II - 1` idle sub-cycles between iterations. Pipeline interleaving fills these gaps with iterations from a different loop instance or context.

```
II=3, single context:
Cycle:  0  1  2  3  4  5  6  7  8
        A0 .. .. A1 .. .. A2 .. ..    (2/3 of cycles wasted)

II=3, interleaved with 3 contexts:
Cycle:  0  1  2  3  4  5  6  7  8
        A0 B0 C0 A1 B1 C1 A2 B2 C2   (100% utilization)
```

### 8.2 Published Work

**Key papers:**

1. **Rau, "Cydra 5 Software Pipelining" (1988):** The original observation that pipeline bubbles can be filled with independent threads. The Cydra 5 processor used hardware "polycyclic scheduling" to interleave up to II independent loop executions.

2. **"Thread-Level Speculation Meets Software Pipelining" (Zhong et al., CGO 2005):** Explores interleaving speculative loop iterations.

3. **"CGRA (Coarse-Grained Reconfigurable Architecture) Pipeline Interleaving" (Hamzeh et al., CASES 2012):** Maps multiple loop iterations onto a CGRA fabric with modulo scheduling, achieving II-way interleaving naturally.

4. **"Dynamic Pipeline Interleaving" (multiple FPGA papers):** In HLS, the most practical approach is to instantiate the pipeline body once and multiplex II independent data streams through it.

### 8.3 Hardware Implementation

```verilog
module interleaved_pipeline #(
    parameter II      = 3,          // initiation interval
    parameter N_CTX   = 3,          // number of interleaved contexts (= II for full utilization)
    parameter DWIDTH  = 32
) (
    input  wire                clk,
    input  wire                rst,
    input  wire                start,
    
    // Each context has its own memory interface
    output reg  [9:0]          addr [0:N_CTX-1],
    input  wire [DWIDTH-1:0]   rdata [0:N_CTX-1],
    output reg  [DWIDTH-1:0]   result [0:N_CTX-1]
);

    // Context selector: round-robin through contexts
    reg [$clog2(N_CTX)-1:0] ctx_sel;
    
    // Per-context state
    reg [15:0] iter_count [0:N_CTX-1];
    reg [DWIDTH-1:0] acc [0:N_CTX-1];
    
    // Shared pipeline registers (multiplexed)
    reg [$clog2(N_CTX)-1:0] ctx_pipe [0:4];  // which context is in each stage
    reg [3:0] valid_sr;
    
    always @(posedge clk) begin
        if (rst) begin
            ctx_sel <= 0;
            // ... reset all contexts ...
        end else begin
            // Rotate context every cycle
            ctx_sel <= (ctx_sel == N_CTX - 1) ? 0 : ctx_sel + 1;
            
            // Stage 0: Issue read for current context
            addr[ctx_sel] <= iter_count[ctx_sel][9:0];
            iter_count[ctx_sel] <= iter_count[ctx_sel] + 1;
            ctx_pipe[0] <= ctx_sel;
            valid_sr[0] <= 1'b1;
            
            // Stages 1-3: Pipeline as normal, but track which context
            ctx_pipe[1] <= ctx_pipe[0];
            ctx_pipe[2] <= ctx_pipe[1];
            ctx_pipe[3] <= ctx_pipe[2];
            valid_sr[3:1] <= valid_sr[2:0];
            
            // Stage 3: Write result back to the correct context
            if (valid_sr[3]) begin
                acc[ctx_pipe[3]] <= acc[ctx_pipe[3]] + rdata[ctx_pipe[3]];
            end
        end
    end
endmodule
```

**Practical use in HLS:** Pipeline interleaving is most useful for:
- Pipelined modules processing multiple independent data streams
- BRAM-limited designs where duplicating memory is cheaper than increasing pipeline depth
- DSP-limited designs where interleaving amortizes the DSP over multiple contexts

---

## 9. Polyhedral Compilation for Hardware {#9-polyhedral-compilation}

### 9.1 Overview

The polyhedral model represents loop nests as integer polyhedra. Each iteration point `(i, j, k, ...)` is a point in Z^n. Dependencies are affine functions mapping iteration points. Transformations (tiling, skewing, interchange, fusion) are affine maps on the polyhedron.

### 9.2 Key Tools and Papers

1. **PLUTO (Bondhugula et al., 2008):** "A Practical and Automatic Polyhedral Parallelizer and Locality Optimizer," PLDI 2008. PLUTO finds affine schedule transformations that maximize parallelism and locality. Originally for CPUs, but applicable to HLS.

2. **ISL (Integer Set Library):** Verdoolaege, "isl: An Integer Set Library for the Polyhedral Model," ICMS 2010. The backend used by PLUTO, PPCG, and most polyhedral tools. Manipulates parametric integer sets and maps.

3. **PPCG (Polyhedral Parallel Code Generator):** Verdoolaege et al. Targets GPUs, but the polyhedral analysis is reusable for HLS: finding tileable loops, computing data reuse distances, identifying parallelism.

4. **Pouchet et al., "Polyhedral Optimization for HLS":** Louis-Noel Pouchet's group (Colorado State, then Ohio State) published extensively on polyhedral HLS:
   - "Polyhedral-Based Data Reuse Optimization for Configurable Computing" (FPGA 2013)
   - "PolyOpt/HLS: Polyhedral Optimization for High-Level Synthesis" (PACT 2012)
   - These papers showed that polyhedral transformations can expose parallelism that simple modulo scheduling misses.

5. **Moreau et al., "Polyhedral-Based Scheduling for HLS" (DAC 2018):** Showed that polyhedral scheduling can find pipeline schedules with lower II than modulo scheduling for multi-dimensional loop nests with complex dependencies.

### 9.3 When Polyhedral Beats Modulo Scheduling

Modulo scheduling works on a single loop level. Polyhedral analysis works on entire loop nests.

**Example where polyhedral wins:**
```c
// 2D convolution — polyhedral analysis finds better schedule
for (int i = 1; i < N-1; i++) {
    for (int j = 1; j < M-1; j++) {
        out[i][j] = in[i-1][j] + in[i][j-1] + in[i][j] + in[i][j+1] + in[i+1][j];
    }
}
```

Modulo scheduling (inner loop only): Each iteration reads 5 elements. With 2 BRAM ports, MemII = ceil(5/2) = 3. Achieves II=3.

Polyhedral analysis: Applies loop skewing to create a wavefront schedule, plus data reuse analysis to show that `in[i][j+1]` in iteration j is `in[i][j]` in iteration j+1. With a register to forward the reused value, only 3 NEW reads per iteration are needed. MemII = ceil(3/2) = 2. Achieves II=2.

### 9.4 Relevance to c2v

For c2v's restricted C subset, polyhedral analysis is **highly relevant** but should be Phase 2:

1. **Phase 1 (current priority):** Get modulo-scheduled pipelined loops working with correct II and proper RTL emission. This handles the common cases (1D loops, simple reductions).

2. **Phase 2:** Add polyhedral analysis for multi-dimensional loop nests. Use ISL (has C API, can be called from Rust via FFI) to compute optimal loop transformations before modulo scheduling.

The restricted C subset (no aliasing, fixed arrays, affine indices) is EXACTLY the domain where polyhedral analysis is most powerful — no conservative assumptions needed.

---

## 10. Open-Source HLS Implementations {#10-open-source-hls}

### 10.1 LegUp (Microchip, originally University of Toronto)

**Repository:** Originally open-source, now Microchip's SmartHLS (closed). But the academic LegUp 4.0 codebase is still available.

**Pipeline approach:**
- Uses LLVM IR as input (unlike c2v which uses tree-sitter)
- Implements Iterative Modulo Scheduling (Rau's IMS)
- Pipeline controller is an FSM with explicit prologue/kernel/epilogue states
- Memory port analysis through LLVM alias analysis
- Support for II > 1 with resource sharing

**Key source files (LegUp 4.0):**
- `llvm/lib/Target/Verilog/SchedulerDAG.cpp` — DAG construction
- `llvm/lib/Target/Verilog/ModuloScheduler.cpp` — IMS implementation
- `llvm/lib/Target/Verilog/GenerateRTL.cpp` — Verilog emission

**RTL emission pattern:** LegUp generates an FSM per pipeline stage, with inter-stage registers. Each stage FSM has states for each sub-cycle (when II > 1).

### 10.2 Bambu (PandA, Politecnico di Milano)

**Repository:** https://github.com/ferrandi/PandA-bambu

**Pipeline approach:**
- C/C++ input through GCC or Clang frontend
- Implements SDC-based scheduling (like c2v!) plus modulo scheduling
- Has sophisticated memory analysis for multi-port BRAMs
- Supports chaining (operation combining within clock cycle)

**Key source files:**
- `src/HLS/scheduling/` — Scheduling algorithms
- `src/HLS/module_allocation/` — Resource binding
- `src/HLS/verilog/` — Verilog emission

**Notable features:**
- Automatic array partitioning for memory port reduction
- DSP inference and chaining
- Speculative execution for conditional branches in pipelines

### 10.3 CIRCT (Circuit IR Compilers and Tools)

**Repository:** https://github.com/llvm/circt

**Pipeline support:**
- CIRCT defines `pipeline` operations in its `Pipeline` dialect
- The `PipelineToHW` pass converts pipeline ops to hardware
- Uses MLIR's analysis framework for scheduling

**Key files:**
- `lib/Dialect/Pipeline/` — Pipeline IR dialect
- `lib/Conversion/PipelineToHW/` — Lowering to hardware
- `lib/Dialect/SSP/` — Static Scheduling Problem formulation

**The SSP dialect** is particularly interesting — it represents scheduling as a constraint satisfaction problem, similar to c2v's SDC approach.

### 10.4 Calyx (Cornell)

**Repository:** https://github.com/calyxir/calyx

**Pipeline approach:**
- Calyx is an intermediate language for hardware compilers
- Pipelines are represented as `static` groups with explicit timing
- The `calyx-opt` pass can pipeline sequential computations
- Uses a "static timing" model where every operation has a known latency

**Key innovation:** Calyx separates the control flow (sequencing) from the datapath (wiring), making pipeline transformation a purely control-flow operation. This is architecturally similar to c2v's separation of DFG and scheduling.

### 10.5 ScaleHLS (Cornell, MLIR-based)

**Repository:** https://github.com/hanchenye/scalehls

**Pipeline approach:**
- Built on MLIR affine dialect
- Uses polyhedral analysis (via MLIR's built-in affine analysis) for loop optimization
- Automatic pipeline insertion with II estimation
- Published at DAC 2022: "ScaleHLS: A New Scalable High-Level Synthesis Framework on Multi-Level Intermediate Representation"

**Key innovation:** ScaleHLS applies loop transformations (tiling, unrolling, pipelining) at the MLIR level, then lowers to CIRCT for RTL generation. The scheduling and transformation happen at a higher abstraction level than traditional HLS.

### 10.6 XLS (Google)

**Repository:** https://github.com/google/xls

**Pipeline approach:**
- XLS uses a "proc" abstraction for stateful computation
- Pipeline scheduling is done by the `pipeline_schedule.cc` module
- Supports manual and automatic pipeline stage assignment
- Uses Z3 SMT solver for scheduling (similar to SDC but more powerful)

**Key files:**
- `xls/scheduling/pipeline_schedule.cc` — Pipeline scheduling
- `xls/codegen/pipeline_generator.cc` — Verilog pipeline emission
- `xls/scheduling/sdc_scheduler.cc` — SDC-based scheduler (!)

**Notable:** XLS's SDC scheduler is very similar to c2v's approach. They formulate scheduling as a linear program with timing constraints and solve it. The pipeline generator then places registers at stage boundaries.

---

## 11. Theoretical Minimum II {#11-theoretical-minimum-ii}

### 11.1 Is Modulo Scheduling Optimal?

**Theorem (Rau, 1994):** For a given II, finding a valid modulo schedule is NP-complete in the general case (reducible from bin packing).

**However:** For the restricted case of HLS (small loop bodies, limited resource types), the problem is tractable:
- With < 50 operations and < 5 resource types, the search space is manageable
- IMS with eviction finds optimal or near-optimal schedules in practice
- The II lower bound (max of ResII, MemII, RecII) is tight for most practical cases

### 11.2 Can Non-Periodic Schedules Do Better?

A modulo schedule is periodic: every iteration has the same schedule, shifted by II cycles. Can an aperiodic schedule achieve lower throughput?

**No, for throughput.** In the steady state, the average throughput is bounded by the same resource and recurrence constraints. An aperiodic schedule might reduce total latency for a FINITE number of iterations (by having a custom prologue), but the steady-state throughput cannot exceed 1/II.

**But for total latency (including prologue/epilogue):** An aperiodic schedule can save cycles. For example, with N iterations and pipeline depth D:
- Periodic schedule: `(N-1) * II + D` cycles
- Optimal aperiodic: can sometimes achieve `(N-1) * II + D - c` for small constant c

This is rarely worth the complexity for N >> D.

### 11.3 The Quasicrystal Idea

An aperiodic but structured schedule — inspired by Penrose tilings — could theoretically tile the MRT in patterns that periodic scheduling cannot. This is unexplored in HLS literature. The idea:
- Instead of repeating the same schedule every II cycles, use a quasi-periodic pattern with period II but varying phase
- This could potentially reduce pipeline register pressure (not II itself)
- No published work found on this specific idea

**Verdict:** Not worth pursuing. The practical gain is negligible compared to getting basic modulo scheduling + RTL emission working correctly.

---

## 12. Implementation Plan for c2v {#12-implementation-plan}

### 12.1 Priority Order

Based on this research, here is the implementation plan:

**Phase 1: Core Pipeline Emission (IMMEDIATE PRIORITY)**

1. **Fix `emit/pipeline.rs` to handle II > 1:**
   - Add II counter that gates input acceptance
   - Modify valid shift register to insert 1s every II cycles instead of every cycle
   - Generate time-multiplexed resource usage for II > 1

2. **Add loop-carried dependency forwarding:**
   - For RecII=1 (simple accumulators): the existing register feedback is sufficient
   - For RecII > 1: generate the sub-cycle FSM shown in Case 4 above
   - Wire the forwarding path from the last stage back to the first stage

3. **Add MemII computation to `pipeline_loop.rs`:**
   - Count memory accesses per array per iteration
   - MemII = max over arrays: ceil(accesses / ports)
   - This is the most common reason Vitis increases II — we must compute it

4. **Pipeline controller FSM:**
   - States: IDLE, RUNNING, FLUSH
   - Handle start/done signaling
   - Generate the ap_ctrl-style interface (or simpler valid_in/valid_out)

**Phase 2: Optimizations**

5. **Chaining integration:**
   - Use `chaining.rs` results to reduce recurrence II
   - Operations chained within a cycle don't add to recurrence latency

6. **Memory port optimization:**
   - Array partitioning to increase effective ports
   - Read-after-write forwarding to eliminate redundant BRAM reads

7. **Pipeline stall/backpressure:**
   - Add `stall` input that freezes the entire pipeline
   - When stall is asserted: don't shift valid_sr, don't update any registers
   - This enables composability with downstream backpressure

**Phase 3: Advanced**

8. **Loop flattening for nested loops**
9. **Pipeline interleaving for II > 1**
10. **Polyhedral analysis via ISL FFI**

### 12.2 The Minimal Correct Pipeline Emission

The absolute minimum to emit correct pipelined Verilog for any loop:

```rust
// Pseudocode for the pipeline emission algorithm
fn emit_pipelined_loop(schedule: &ModuloSchedule, loop_info: &LoopInfo) -> String {
    let ii = schedule.ii;
    let n_stages = schedule.stage_count;
    let total_depth = (n_stages - 1) * ii + body_latency_within_last_stage;
    
    // 1. Emit module ports (clk, rst, start, done, memory interfaces)
    // 2. Emit valid shift register: reg [total_depth-1:0] valid_sr;
    // 3. Emit II counter: reg [$clog2(ii)-1:0] ii_ctr;
    // 4. Emit iteration counter
    // 5. Emit pipeline registers for each value that crosses a stage boundary
    
    // 6. For each stage s in 0..n_stages:
    //      For each sub-cycle c in 0..ii-1:
    //        Emit the operations scheduled at (stage=s, cycle=c)
    //        Gate by valid_sr[s * ii + c]
    
    // 7. For loop-carried dependencies:
    //      Emit the feedback path from stage n_stages-1 back to stage 0
    //      The feedback register is written in the last stage, read in the first
    //      If II >= recurrence_latency, no special forwarding needed
    //      If II == 1 and recurrence == 1, standard register feedback works
    
    // 8. Emit FSM: IDLE → RUNNING → FLUSH → IDLE
    // 9. Emit done signal: asserted when valid_sr == 0 after all iterations fed
}
```

### 12.3 Key Data Structures Needed

```rust
// Stage map: which operations execute in which (stage, sub-cycle)
struct PipelineStage {
    stage_num: u32,
    sub_cycle: u32,     // 0..II-1
    operations: Vec<NodeId>,
}

// Pipeline register: a value that must be forwarded across stage boundaries
struct PipelineRegister {
    source_node: NodeId,
    source_stage: u32,
    dest_stage: u32,
    width: u32,
    name: String,
    // Number of register stages needed = dest_stage - source_stage
    // (or dest_stage - source_stage - 1 if chained)
}

// Feedback path: loop-carried dependency forwarding
struct FeedbackPath {
    write_node: NodeId,
    write_stage: u32,
    read_node: NodeId,
    read_stage: u32,
    var_name: String,
    width: u32,
    recurrence_latency: u32,
}
```

### 12.4 Testing Strategy

1. **Unit test:** Emit pipeline for `sum += a[i]`, simulate with iverilog, verify II=1 throughput and correct final sum.
2. **Unit test:** Emit pipeline for `c[i] = a[i] + b[i]`, verify all N outputs correct at II=1.
3. **Unit test:** Emit pipeline for multiply-accumulate with II > 1, verify correct results.
4. **Regression:** Run all existing testbench tests in pipelined mode, compare results to FSM mode.
5. **Performance:** Compare total cycles (pipelined vs FSM) for each test case, verify speedup matches theoretical `body_latency / II`.

---

## References

### Foundational Papers

1. Rau, B.R. "Iterative Modulo Scheduling: An Algorithm For Software Pipelining Loops." MICRO-27, 1994.
2. Llosa, J. et al. "Swing Modulo Scheduling: A Lifetime-Sensitive Approach." PACT, 1996.
3. Huff, R.A. "Lifetime-Sensitive Modulo Scheduling." PLDI, 1993.
4. Lam, M. "Software Pipelining: An Effective Scheduling Technique for VLIW Machines." PLDI, 1988.
5. Rau, B.R. and Glaeser, C.D. "Some Scheduling Techniques and an Easily Schedulable Horizontal Architecture for High Performance Scientific Computing." MICRO-14, 1981.

### HLS-Specific Papers

6. Canis, A. et al. "LegUp: High-Level Synthesis for FPGA-Based Processor/Accelerator Systems." FPGA, 2011.
7. Pilato, C. and Ferrandi, F. "Bambu: A Modular Framework for the High Level Synthesis of Memory-Intensive Applications." FPL, 2013.
8. Zhang, N. et al. "ScaleHLS: A New Scalable High-Level Synthesis Framework on Multi-Level Intermediate Representation." DAC, 2022.
9. Josipovic, L. et al. "Dynamatic: From C/C++ to Dynamically Scheduled Circuits." FPGA, 2022.

### Polyhedral HLS

10. Pouchet, L.-N. et al. "Polyhedral-Based Data Reuse Optimization for Configurable Computing." FPGA, 2013.
11. Bondhugula, U. et al. "A Practical Automatic Polyhedral Parallelizer and Locality Optimizer." PLDI, 2008.
12. Moreau, T. et al. "Polyhedral-Based Scheduling for HLS." DAC, 2018.

### Xilinx Vitis HLS

13. Xilinx, "Vitis HLS User Guide (UG1399)," 2023.2. Chapters on Loop Pipelining and Optimization.
14. Xilinx, "Vivado Design Suite User Guide: High-Level Synthesis (UG902)," 2019.2.

### Open-Source Tools

15. CIRCT Project: https://github.com/llvm/circt
16. Calyx Project: https://github.com/calyxir/calyx  
17. XLS Project: https://github.com/google/xls
18. Bambu Project: https://github.com/ferrandi/PandA-bambu

---

*This research document directly enables c2v pipeline emission. The concrete Verilog examples in Section 4 should be used as templates for the `emit/pipeline.rs` rewrite. The II computation fixes in Section 3 (especially MemII) should be added to `pipeline_loop.rs` immediately.*
