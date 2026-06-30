use std::fmt::Write;
use crate::ir::{BoundDfg, DfgOp, NodeId};

pub fn emit_streaming_accelerator( bound: &BoundDfg, input_array: &str, output_array: &str ) -> String {
    let mut out = String::with_capacity( 8192 );
    let dfg       = &bound.scheduled.dfg;
    let name      = super::verilog_safe_name( &dfg.name );
    let sig_names = super::build_signal_names( bound );
    let depth     = bound.scheduled.total_cycles;

    let has_accum = dfg.nodes.iter().any(|n| matches!(&n.op, DfgOp::Add | DfgOp::Sub));

    writeln!( out, "// c2v streaming BRAM accelerator: {name}" ).unwrap();
    writeln!( out, "// Pipelined II=1 with BRAM read/write and accumulator forwarding" ).unwrap();
    writeln!( out, "// Body latency: {depth} cycles" ).unwrap();
    writeln!( out, "// Throughput: 1 element per clock (after pipeline fill)" ).unwrap();
    writeln!( out ).unwrap();

    writeln!( out, "module {name}_stream #(" ).unwrap();
    writeln!( out, "    parameter N = 1024," ).unwrap();
    writeln!( out, "    parameter AWIDTH = 10," ).unwrap();
    writeln!( out, "    parameter DWIDTH = 32" ).unwrap();
    writeln!( out, ")(" ).unwrap();
    writeln!( out, "    input  wire              clk," ).unwrap();
    writeln!( out, "    input  wire              rst," ).unwrap();
    writeln!( out, "    input  wire              start," ).unwrap();
    writeln!( out, "    output reg               done," ).unwrap();
    writeln!( out, "    output reg               idle," ).unwrap();
    writeln!( out, "    output reg  [AWIDTH-1:0] {input_array}_addr," ).unwrap();
    writeln!( out, "    input  wire [DWIDTH-1:0] {input_array}_rdata," ).unwrap();
    writeln!( out, "    output reg  [AWIDTH-1:0] {output_array}_addr," ).unwrap();
    writeln!( out, "    output reg  [DWIDTH-1:0] {output_array}_wdata," ).unwrap();
    writeln!( out, "    output reg               {output_array}_we" ).unwrap();
    writeln!( out, ");" ).unwrap();
    writeln!( out ).unwrap();

    let pipe_depth = depth + 2;
    writeln!( out, "    // Pipeline: {pipe_depth} stages (2 BRAM latency + {depth} compute)" ).unwrap();
    writeln!( out, "    reg [{pd}:0] valid_sr;", pd = pipe_depth - 1 ).unwrap();
    writeln!( out, "    reg [AWIDTH:0] iter_count;" ).unwrap();
    writeln!( out, "    wire feeding = (iter_count < N);" ).unwrap();
    writeln!( out ).unwrap();

    writeln!( out, "    // Pipeline index registers (track which element is in which stage)" ).unwrap();
    for s in 0..pipe_depth {
        writeln!( out, "    reg [AWIDTH-1:0] idx_s{s};" ).unwrap();
    }
    writeln!( out ).unwrap();

    writeln!( out, "    // BRAM read capture" ).unwrap();
    writeln!( out, "    reg [DWIDTH-1:0] rdata_captured;" ).unwrap();
    writeln!( out ).unwrap();

    writeln!( out, "    // Compute pipeline registers" ).unwrap();
    for node in &dfg.nodes {
        match &node.op {
            DfgOp::Input(_) | DfgOp::Const(_) | DfgOp::Output(_) | DfgOp::Nop => {}
            _ => {
                if let Some(sig) = sig_names.get( &node.id ) {
                    let wd = super::width_decl( node.width );
                    let sign = if node.signed { "signed " } else { "" };
                    if node.latency > 0 {
                        writeln!( out, "    reg  {sign}{wd}{sig};" ).unwrap();
                    } else {
                        writeln!( out, "    wire {sign}{wd}{sig};" ).unwrap();
                    }
                }
            }
        }
    }
    for node in &dfg.nodes {
        if node.latency != 0 { continue; }
        if matches!(&node.op, DfgOp::Input(_) | DfgOp::Const(_) | DfgOp::Output(_) | DfgOp::Nop) { continue; }
        if let Some(sig) = sig_names.get(&node.id) {
            let inputs = super::get_inputs(node.id, bound);
            match &node.op {
                DfgOp::Not if !inputs.is_empty() => {
                    let a = pipe_operand(inputs[0], &sig_names, bound);
                    writeln!(out, "    assign {sig} = ~{a};").unwrap();
                }
                DfgOp::Mux if inputs.len() >= 3 => {
                    let c = pipe_operand(inputs[0], &sig_names, bound);
                    let t = pipe_operand(inputs[1], &sig_names, bound);
                    let f = pipe_operand(inputs[2], &sig_names, bound);
                    writeln!(out, "    assign {sig} = {c} ? {t} : {f};").unwrap();
                }
                _ if super::verilog_binop(&node.op).is_some() && inputs.len() >= 2 => {
                    let op = super::verilog_binop(&node.op).unwrap();
                    let l = pipe_operand(inputs[0], &sig_names, bound);
                    let r = pipe_operand(inputs[1], &sig_names, bound);
                    writeln!(out, "    assign {sig} = {l} {op} {r};").unwrap();
                }
                _ => {}
            }
        }
    }
    writeln!( out ).unwrap();

    writeln!( out, "    // Accumulator with forwarding path" ).unwrap();
    writeln!( out, "    reg [DWIDTH-1:0] accum_reg;" ).unwrap();
    writeln!( out, "    reg              accum_valid;" ).unwrap();
    writeln!( out ).unwrap();

    writeln!( out, "    reg [1:0] state;" ).unwrap();
    writeln!( out, "    localparam S_IDLE = 0, S_RUN = 1, S_FLUSH = 2;" ).unwrap();
    writeln!( out ).unwrap();

    writeln!( out, "    always @(posedge clk) begin" ).unwrap();
    writeln!( out, "        if (rst) begin" ).unwrap();
    writeln!( out, "            state       <= S_IDLE;" ).unwrap();
    writeln!( out, "            valid_sr    <= 0;" ).unwrap();
    writeln!( out, "            iter_count  <= 0;" ).unwrap();
    writeln!( out, "            done        <= 1'b0;" ).unwrap();
    writeln!( out, "            idle        <= 1'b1;" ).unwrap();
    writeln!( out, "            {output_array}_we <= 1'b0;" ).unwrap();
    writeln!( out, "            accum_reg   <= 0;" ).unwrap();
    writeln!( out, "            accum_valid <= 1'b0;" ).unwrap();
    writeln!( out, "        end else begin" ).unwrap();
    writeln!( out, "            {output_array}_we <= 1'b0;" ).unwrap();
    writeln!( out, "            case (state)" ).unwrap();

    writeln!( out, "                S_IDLE: begin" ).unwrap();
    writeln!( out, "                    done <= 1'b0; idle <= 1'b1;" ).unwrap();
    writeln!( out, "                    if (start) begin" ).unwrap();
    writeln!( out, "                        state <= S_RUN; iter_count <= 0; idle <= 1'b0;" ).unwrap();
    writeln!( out, "                        accum_reg <= 0; accum_valid <= 1'b0;" ).unwrap();
    writeln!( out, "                    end" ).unwrap();
    writeln!( out, "                end" ).unwrap();

    writeln!( out, "                S_RUN: begin" ).unwrap();

    writeln!( out, "                    // Stage 0: Issue BRAM read address (II=1)" ).unwrap();
    writeln!( out, "                    if (feeding) begin" ).unwrap();
    writeln!( out, "                        {input_array}_addr <= iter_count[AWIDTH-1:0];" ).unwrap();
    writeln!( out, "                        idx_s0 <= iter_count[AWIDTH-1:0];" ).unwrap();
    writeln!( out, "                        iter_count <= iter_count + 1;" ).unwrap();
    writeln!( out, "                    end" ).unwrap();

    writeln!( out, "                    // Valid shift register" ).unwrap();
    if pipe_depth > 1 {
        writeln!( out, "                    valid_sr <= {{valid_sr[{pd}:0], feeding ? 1'b1 : 1'b0}};",
            pd = pipe_depth - 2 ).unwrap();
    }

    writeln!( out, "                    // Index forwarding through pipeline stages" ).unwrap();
    for s in 1..pipe_depth {
        writeln!( out, "                    idx_s{s} <= idx_s{p};", p = s - 1 ).unwrap();
    }

    writeln!( out, "                    // Stage 1: BRAM data arrives" ).unwrap();
    writeln!( out, "                    rdata_captured <= {input_array}_rdata;" ).unwrap();

    writeln!( out, "                    // Stages 2+: Compute with forwarded accumulator" ).unwrap();
    for node in &dfg.nodes {
        match &node.op {
            DfgOp::Input(_) | DfgOp::Const(_) | DfgOp::Output(_) | DfgOp::Nop => {}
            _ if node.latency > 0 => {
                if let Some(sig) = sig_names.get( &node.id ) {
                    let inputs = super::get_inputs( node.id, bound );
                    match &node.op {
                        DfgOp::Add if inputs.len() >= 2 => {
                            let l = pipe_operand( inputs[0], &sig_names, bound );
                            let r = pipe_operand( inputs[1], &sig_names, bound );
                            writeln!( out, "                    {sig} <= {l} + {r};" ).unwrap();
                            writeln!( out, "                    accum_reg <= {l} + {r};" ).unwrap();
                            writeln!( out, "                    accum_valid <= 1'b1;" ).unwrap();
                        }
                        DfgOp::Sub if inputs.len() >= 2 => {
                            let l = pipe_operand( inputs[0], &sig_names, bound );
                            let r = pipe_operand( inputs[1], &sig_names, bound );
                            writeln!( out, "                    {sig} <= {l} - {r};" ).unwrap();
                        }
                        DfgOp::Mul if inputs.len() >= 2 => {
                            let l = pipe_operand( inputs[0], &sig_names, bound );
                            let r = pipe_operand( inputs[1], &sig_names, bound );
                            writeln!( out, "                    {sig} <= {l} * {r};" ).unwrap();
                        }
                        _ => {
                            if let Some(op_str) = super::verilog_binop( &node.op ) {
                                if inputs.len() >= 2 {
                                    let l = pipe_operand( inputs[0], &sig_names, bound );
                                    let r = pipe_operand( inputs[1], &sig_names, bound );
                                    writeln!( out, "                    {sig} <= {l} {op_str} {r};" ).unwrap();
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    writeln!( out, "                    // Output stage: write to BRAM" ).unwrap();
    writeln!( out, "                    if (valid_sr[{pd}]) begin", pd = pipe_depth - 1 ).unwrap();
    let output_sig = dfg.outputs.first()
        .map(|p| super::verilog_safe_name(&p.name))
        .unwrap_or_else(|| "result".to_string());
    let last_compute = dfg.nodes.iter().rev()
        .find(|n| n.latency > 0 && !matches!(&n.op, DfgOp::Input(_) | DfgOp::Const(_) | DfgOp::Output(_) | DfgOp::Nop))
        .and_then(|n| sig_names.get(&n.id))
        .cloned()
        .unwrap_or_else(|| "rdata_captured".to_string());
    writeln!( out, "                        {output_array}_addr  <= idx_s{pd}[AWIDTH-1:0];", pd = pipe_depth - 1 ).unwrap();
    writeln!( out, "                        {output_array}_wdata <= {last_compute};" ).unwrap();
    writeln!( out, "                        {output_array}_we    <= 1'b1;" ).unwrap();
    writeln!( out, "                    end" ).unwrap();

    writeln!( out, "                    if (!feeding && valid_sr == 0) begin" ).unwrap();
    writeln!( out, "                        state <= S_IDLE; done <= 1'b1; idle <= 1'b1;" ).unwrap();
    writeln!( out, "                    end else if (!feeding) begin" ).unwrap();
    writeln!( out, "                        state <= S_FLUSH;" ).unwrap();
    writeln!( out, "                    end" ).unwrap();
    writeln!( out, "                end" ).unwrap();

    writeln!( out, "                S_FLUSH: begin" ).unwrap();
    if pipe_depth > 1 {
        writeln!( out, "                    valid_sr <= {{valid_sr[{pd}:0], 1'b0}};", pd = pipe_depth - 2 ).unwrap();
    }
    for s in 1..pipe_depth {
        writeln!( out, "                    idx_s{s} <= idx_s{p};", p = s - 1 ).unwrap();
    }
    writeln!( out, "                    rdata_captured <= {input_array}_rdata;" ).unwrap();
    writeln!( out, "                    if (valid_sr[{pd}]) begin", pd = pipe_depth - 1 ).unwrap();
    writeln!( out, "                        {output_array}_addr  <= idx_s{pd}[AWIDTH-1:0];", pd = pipe_depth - 1 ).unwrap();
    writeln!( out, "                        {output_array}_wdata <= {last_compute};" ).unwrap();
    writeln!( out, "                        {output_array}_we    <= 1'b1;" ).unwrap();
    writeln!( out, "                    end" ).unwrap();
    writeln!( out, "                    if (valid_sr == 0) begin" ).unwrap();
    writeln!( out, "                        state <= S_IDLE; done <= 1'b1; idle <= 1'b1;" ).unwrap();
    writeln!( out, "                    end" ).unwrap();
    writeln!( out, "                end" ).unwrap();

    writeln!( out, "                default: state <= S_IDLE;" ).unwrap();
    writeln!( out, "            endcase" ).unwrap();
    writeln!( out, "        end" ).unwrap();
    writeln!( out, "    end" ).unwrap();
    writeln!( out ).unwrap();
    writeln!( out, "endmodule" ).unwrap();

    out
}

fn pipe_operand( node_id: NodeId, sig_names: &std::collections::HashMap<NodeId, String>, bound: &BoundDfg ) -> String {
    let node = &bound.scheduled.dfg.nodes[node_id as usize];
    match &node.op {
        DfgOp::Input(_) => "rdata_captured".to_string(),
        DfgOp::Const(val) => {
            let w = node.width;
            format!("{w}'d{val}")
        }
        _ => sig_names.get(&node_id).cloned().unwrap_or_else(|| format!("node_{node_id}")),
    }
}
