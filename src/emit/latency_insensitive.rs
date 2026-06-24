/* c2v_latency_insensitive generates a latency-insensitive wrapper
   for any c2v module. LI wrappers decouple the timing of the
   producer from the consumer by adding elastic buffers with
   valid/ready handshaking at every port.

   This is critical for composing multiple HLS-generated modules:
   if module A feeds module B and their latencies change (e.g.,
   after re-synthesis with different constraints), the LI wrapper
   ensures they still compose correctly without changing the
   interconnect.

   Protocol:
     - Each input  gets: valid_in_{name}, ready_out_{name}
     - Each output gets: valid_out_{name}, ready_in_{name}
     - The wrapper stalls the FSM when any input isn't valid
     - Outputs are held until acknowledged by downstream ready */

use std::fmt::Write;

use crate::ir::BoundDfg;

pub fn emit_latency_insensitive_wrapper( bound: &BoundDfg ) -> String {
    let mut out = String::with_capacity( 8192 );
    let dfg  = &bound.scheduled.dfg;
    let name = super::verilog_safe_name( &dfg.name );

    writeln!( out, "// c2v latency-insensitive wrapper for {name}" ).unwrap();
    writeln!( out ).unwrap();

    /* Module declaration */
    writeln!( out, "module {name}_li (" ).unwrap();
    writeln!( out, "    input  wire              clk," ).unwrap();
    writeln!( out, "    input  wire              rst," ).unwrap();
    writeln!( out ).unwrap();

    /* Input channels — valid/ready + data */
    for port in &dfg.inputs {
        let n  = super::verilog_safe_name( &port.name );
        let wd = super::width_decl( port.width );
        writeln!( out, "    input  wire {wd}{n}," ).unwrap();
        writeln!( out, "    input  wire              {n}_valid," ).unwrap();
        writeln!( out, "    output wire              {n}_ready," ).unwrap();
    }

    /* Output channels */
    for (i, port) in dfg.outputs.iter().enumerate() {
        let n  = super::verilog_safe_name( &port.name );
        let wd = super::width_decl( port.width );
        writeln!( out, "    output wire {wd}{n}," ).unwrap();
        writeln!( out, "    output wire              {n}_valid," ).unwrap();
        let comma = if i == dfg.outputs.len() - 1 { "" } else { "," };
        writeln!( out, "    input  wire              {n}_ready{comma}" ).unwrap();
    }
    writeln!( out, ");" ).unwrap();
    writeln!( out ).unwrap();

    /* All inputs valid? */
    let input_names: Vec<String> = dfg.inputs.iter()
        .map(|p| super::verilog_safe_name(&p.name))
        .collect();
    let output_names: Vec<String> = dfg.outputs.iter()
        .map(|p| super::verilog_safe_name(&p.name))
        .collect();

    write!( out, "    wire all_inputs_valid = " ).unwrap();
    if input_names.is_empty() {
        writeln!( out, "1'b1;" ).unwrap();
    } else {
        for (i, n) in input_names.iter().enumerate() {
            if i > 0 { write!( out, " && " ).unwrap(); }
            write!( out, "{n}_valid" ).unwrap();
        }
        writeln!( out, ";" ).unwrap();
    }

    /* All outputs consumed? */
    writeln!( out, "    reg  outputs_valid;" ).unwrap();
    write!( out, "    wire all_outputs_consumed = " ).unwrap();
    if output_names.is_empty() {
        writeln!( out, "1'b1;" ).unwrap();
    } else {
        for (i, n) in output_names.iter().enumerate() {
            if i > 0 { write!( out, " && " ).unwrap(); }
            write!( out, "(!outputs_valid || {n}_ready)" ).unwrap();
        }
        writeln!( out, ";" ).unwrap();
    }
    writeln!( out ).unwrap();

    /* Core signals */
    writeln!( out, "    wire core_start = all_inputs_valid && !outputs_valid;" ).unwrap();
    writeln!( out, "    wire core_done;" ).unwrap();
    writeln!( out ).unwrap();

    /* Core instantiation */
    for port in &dfg.outputs {
        let n  = super::verilog_safe_name( &port.name );
        let wd = super::width_decl( port.width );
        writeln!( out, "    wire {wd}core_{n};" ).unwrap();
    }
    writeln!( out ).unwrap();

    writeln!( out, "    {name} core (" ).unwrap();
    writeln!( out, "        .clk(clk), .rst(rst)," ).unwrap();
    writeln!( out, "        .start(core_start), .done(core_done)," ).unwrap();
    let all_ports: Vec<_> = dfg.inputs.iter().chain(dfg.outputs.iter()).collect();
    for (i, port) in all_ports.iter().enumerate() {
        let n     = super::verilog_safe_name( &port.name );
        let comma = if i == all_ports.len() - 1 { "" } else { "," };
        let conn  = match port.direction {
            crate::ir::Direction::In  => n.clone(),
            crate::ir::Direction::Out => format!("core_{n}"),
        };
        writeln!( out, "        .{n}({conn}){comma}" ).unwrap();
    }
    writeln!( out, "    );" ).unwrap();
    writeln!( out ).unwrap();

    /* Ready signals: we accept inputs when we're idle */
    for n in &input_names {
        writeln!( out, "    assign {n}_ready = !outputs_valid;" ).unwrap();
    }
    writeln!( out ).unwrap();

    /* Output hold registers */
    for port in &dfg.outputs {
        let n  = super::verilog_safe_name( &port.name );
        let wd = super::width_decl( port.width );
        writeln!( out, "    reg  {wd}{n}_hold;" ).unwrap();
    }
    writeln!( out ).unwrap();

    /* Output valid/data logic */
    writeln!( out, "    always @(posedge clk) begin" ).unwrap();
    writeln!( out, "        if (rst) begin" ).unwrap();
    writeln!( out, "            outputs_valid <= 1'b0;" ).unwrap();
    for n in &output_names {
        writeln!( out, "            {n}_hold <= 0;" ).unwrap();
    }
    writeln!( out, "        end else begin" ).unwrap();
    writeln!( out, "            if (core_done) begin" ).unwrap();
    writeln!( out, "                outputs_valid <= 1'b1;" ).unwrap();
    for port in &dfg.outputs {
        let n = super::verilog_safe_name( &port.name );
        writeln!( out, "                {n}_hold <= core_{n};" ).unwrap();
    }
    writeln!( out, "            end else if (all_outputs_consumed) begin" ).unwrap();
    writeln!( out, "                outputs_valid <= 1'b0;" ).unwrap();
    writeln!( out, "            end" ).unwrap();
    writeln!( out, "        end" ).unwrap();
    writeln!( out, "    end" ).unwrap();
    writeln!( out ).unwrap();

    /* Output assignments */
    for n in &output_names {
        writeln!( out, "    assign {n} = {n}_hold;" ).unwrap();
        writeln!( out, "    assign {n}_valid = outputs_valid;" ).unwrap();
    }
    writeln!( out ).unwrap();

    writeln!( out, "endmodule" ).unwrap();
    out
}
