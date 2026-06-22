/* c2v_axi_stream generates an AXI-Stream wrapper around a c2v-compiled
   module. The wrapper converts between the module's start/done handshake
   and the standard AXI-Stream TVALID/TREADY/TDATA protocol. The input
   stream feeds operands, the output stream emits results. This allows
   any c2v module to plug directly into a Vivado block design, connect
   to AXI DMA, or chain with other stream-based IP. */

use std::fmt::Write;

use crate::ir::BoundDfg;

/* c2v_axi_stream_emit generates the complete AXI-Stream wrapper Verilog
   for the given bound DFG. Returns a string containing the wrapper
   module definition. The wrapper instantiates the inner c2v module and
   manages flow control via a simple state machine:

   IDLE → LOAD (accept input) → RUN (compute) → EMIT (output result)

   The input TDATA is split across the module's input ports. The output
   TDATA is the concatenation of all output ports. */

pub fn emit_axi_stream_wrapper( bound: &BoundDfg ) -> String {
    let mut out = String::with_capacity( 8192 );
    let dfg    = &bound.scheduled.dfg;
    let name   = super::verilog_safe_name( &dfg.name );

    /* Compute bus widths */
    let in_width:  u32 = dfg.inputs.iter().map( |p| p.width ).sum();
    let out_width: u32 = dfg.outputs.iter().map( |p| p.width ).sum();

    writeln!( out, "// c2v AXI-Stream wrapper for {name}" ).unwrap();
    writeln!( out, "// Input  TDATA width: {in_width} bits" ).unwrap();
    writeln!( out, "// Output TDATA width: {out_width} bits" ).unwrap();
    writeln!( out, "// Latency: {} cycles", bound.scheduled.total_cycles ).unwrap();
    writeln!( out ).unwrap();

    writeln!( out, "module {name}_axis (" ).unwrap();
    writeln!( out, "    input  wire              aclk," ).unwrap();
    writeln!( out, "    input  wire              aresetn," ).unwrap();
    writeln!( out ).unwrap();

    /* AXI-Stream slave (input) */
    writeln!( out, "    /* AXI-Stream slave (input) */" ).unwrap();
    writeln!( out, "    input  wire [{w}:0] s_axis_tdata,",  w = in_width - 1 ).unwrap();
    writeln!( out, "    input  wire              s_axis_tvalid," ).unwrap();
    writeln!( out, "    output reg               s_axis_tready," ).unwrap();
    writeln!( out ).unwrap();

    /* AXI-Stream master (output) */
    writeln!( out, "    /* AXI-Stream master (output) */" ).unwrap();
    writeln!( out, "    output reg  [{w}:0] m_axis_tdata,", w = out_width - 1 ).unwrap();
    writeln!( out, "    output reg               m_axis_tvalid," ).unwrap();
    writeln!( out, "    input  wire              m_axis_tready" ).unwrap();
    writeln!( out, ");" ).unwrap();
    writeln!( out ).unwrap();

    /* Internal signals */
    writeln!( out, "    /* Internal signals */" ).unwrap();
    writeln!( out, "    reg              core_start;" ).unwrap();
    writeln!( out, "    wire             core_done;" ).unwrap();
    writeln!( out, "    wire             rst;" ).unwrap();
    writeln!( out ).unwrap();

    for port in &dfg.inputs {
        let n  = super::verilog_safe_name( &port.name );
        let wd = super::width_decl( port.width );
        writeln!( out, "    reg  {wd}{n}_reg;" ).unwrap();
    }

    for port in &dfg.outputs {
        let n  = super::verilog_safe_name( &port.name );
        let wd = super::width_decl( port.width );
        writeln!( out, "    wire {wd}{n}_wire;" ).unwrap();
    }
    writeln!( out ).unwrap();

    writeln!( out, "    assign rst = ~aresetn;" ).unwrap();
    writeln!( out ).unwrap();

    /* FSM states */
    writeln!( out, "    localparam [1:0] S_IDLE = 2'd0;" ).unwrap();
    writeln!( out, "    localparam [1:0] S_LOAD = 2'd1;" ).unwrap();
    writeln!( out, "    localparam [1:0] S_RUN  = 2'd2;" ).unwrap();
    writeln!( out, "    localparam [1:0] S_EMIT = 2'd3;" ).unwrap();
    writeln!( out, "    reg [1:0] axi_state;" ).unwrap();
    writeln!( out ).unwrap();

    /* Core module instantiation */
    writeln!( out, "    /* Core compute module */" ).unwrap();
    writeln!( out, "    {name} core (" ).unwrap();
    writeln!( out, "        .clk   ( aclk )," ).unwrap();
    writeln!( out, "        .rst   ( rst )," ).unwrap();
    writeln!( out, "        .start ( core_start )," ).unwrap();
    writeln!( out, "        .done  ( core_done )," ).unwrap();

    let all_ports: Vec<_> = dfg.inputs.iter().chain( dfg.outputs.iter() ).collect();
    for (i, port) in all_ports.iter().enumerate() {
        let n     = super::verilog_safe_name( &port.name );
        let comma = if i == all_ports.len() - 1 { "" } else { "," };
        let suffix = match port.direction {
            crate::ir::Direction::In  => "_reg",
            crate::ir::Direction::Out => "_wire",
        };
        writeln!( out, "        .{n} ( {n}{suffix} ){comma}" ).unwrap();
    }
    writeln!( out, "    );" ).unwrap();
    writeln!( out ).unwrap();

    /* Unpack s_axis_tdata into input registers */
    writeln!( out, "    /* Unpack input stream into port registers */" ).unwrap();
    let mut bit_offset = 0u32;
    writeln!( out, "    always @(posedge aclk) begin" ).unwrap();
    writeln!( out, "        if (rst) begin" ).unwrap();
    for port in &dfg.inputs {
        let n = super::verilog_safe_name( &port.name );
        writeln!( out, "            {n}_reg <= 0;" ).unwrap();
    }
    writeln!( out, "        end else if (axi_state == S_LOAD && s_axis_tvalid) begin" ).unwrap();
    for port in &dfg.inputs {
        let n = super::verilog_safe_name( &port.name );
        let hi = bit_offset + port.width - 1;
        writeln!( out, "            {n}_reg <= s_axis_tdata[{hi}:{bit_offset}];" ).unwrap();
        bit_offset += port.width;
    }
    writeln!( out, "        end" ).unwrap();
    writeln!( out, "    end" ).unwrap();
    writeln!( out ).unwrap();

    /* Pack output wires into m_axis_tdata */
    writeln!( out, "    /* Pack output wires into output stream */" ).unwrap();
    let mut out_parts = Vec::new();
    for port in dfg.outputs.iter().rev() {
        let n = super::verilog_safe_name( &port.name );
        out_parts.push( format!( "{n}_wire" ) );
    }
    if out_parts.len() == 1 {
        writeln!( out, "    wire [{w}:0] packed_result = {p};",
            w = out_width - 1, p = out_parts[0] ).unwrap();
    } else {
        writeln!( out, "    wire [{w}:0] packed_result = {{{p}}};",
            w = out_width - 1, p = out_parts.join( ", " ) ).unwrap();
    }
    writeln!( out ).unwrap();

    /* AXI-Stream FSM */
    writeln!( out, "    /* AXI-Stream flow control FSM */" ).unwrap();
    let wait_cycles = bound.scheduled.total_cycles * 2 + 4;
    writeln!( out, "    reg [7:0] wait_counter;" ).unwrap();
    writeln!( out ).unwrap();
    writeln!( out, "    always @(posedge aclk) begin" ).unwrap();
    writeln!( out, "        if (rst) begin" ).unwrap();
    writeln!( out, "            axi_state    <= S_IDLE;" ).unwrap();
    writeln!( out, "            s_axis_tready <= 1'b0;" ).unwrap();
    writeln!( out, "            m_axis_tvalid <= 1'b0;" ).unwrap();
    writeln!( out, "            core_start    <= 1'b0;" ).unwrap();
    writeln!( out, "            wait_counter  <= 0;" ).unwrap();
    writeln!( out, "        end else begin" ).unwrap();
    writeln!( out, "            case (axi_state)" ).unwrap();

    writeln!( out, "                S_IDLE: begin" ).unwrap();
    writeln!( out, "                    s_axis_tready <= 1'b1;" ).unwrap();
    writeln!( out, "                    m_axis_tvalid <= 1'b0;" ).unwrap();
    writeln!( out, "                    axi_state     <= S_LOAD;" ).unwrap();
    writeln!( out, "                end" ).unwrap();

    writeln!( out, "                S_LOAD: begin" ).unwrap();
    writeln!( out, "                    if (s_axis_tvalid) begin" ).unwrap();
    writeln!( out, "                        s_axis_tready <= 1'b0;" ).unwrap();
    writeln!( out, "                        core_start    <= 1'b1;" ).unwrap();
    writeln!( out, "                        wait_counter  <= 0;" ).unwrap();
    writeln!( out, "                        axi_state     <= S_RUN;" ).unwrap();
    writeln!( out, "                    end" ).unwrap();
    writeln!( out, "                end" ).unwrap();

    writeln!( out, "                S_RUN: begin" ).unwrap();
    writeln!( out, "                    core_start <= 1'b0;" ).unwrap();
    writeln!( out, "                    if (core_done) begin" ).unwrap();
    writeln!( out, "                        m_axis_tdata  <= packed_result;" ).unwrap();
    writeln!( out, "                        m_axis_tvalid <= 1'b1;" ).unwrap();
    writeln!( out, "                        axi_state     <= S_EMIT;" ).unwrap();
    writeln!( out, "                    end" ).unwrap();
    writeln!( out, "                end" ).unwrap();

    writeln!( out, "                S_EMIT: begin" ).unwrap();
    writeln!( out, "                    if (m_axis_tready) begin" ).unwrap();
    writeln!( out, "                        m_axis_tvalid <= 1'b0;" ).unwrap();
    writeln!( out, "                        axi_state     <= S_IDLE;" ).unwrap();
    writeln!( out, "                    end" ).unwrap();
    writeln!( out, "                end" ).unwrap();

    writeln!( out, "                default: axi_state <= S_IDLE;" ).unwrap();
    writeln!( out, "            endcase" ).unwrap();
    writeln!( out, "        end" ).unwrap();
    writeln!( out, "    end" ).unwrap();
    writeln!( out ).unwrap();

    writeln!( out, "endmodule" ).unwrap();

    out
}
