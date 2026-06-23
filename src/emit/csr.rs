/* c2v_csr generates a Control/Status Register (CSR) block that
   provides a software-accessible memory-mapped interface to the
   c2v module. This lets a CPU (soft-core or hard-core) configure
   inputs, trigger computation, and read results via register
   writes/reads. Essential for any SoC integration.

   Register map:
     0x00: CTRL    [W]  bit 0 = start
     0x04: STATUS  [R]  bit 0 = done, bit 1 = busy
     0x08: INPUT0  [W]  first input word
     0x0C: INPUT1  [W]  second input word (if needed)
     ...
     0x40: OUTPUT0 [R]  first output word
     0x44: OUTPUT1 [R]  second output word (if needed)  */

use std::fmt::Write;

use crate::ir::BoundDfg;

pub fn emit_csr_block( bound: &BoundDfg ) -> String {
    let mut out = String::with_capacity( 8192 );
    let dfg    = &bound.scheduled.dfg;
    let name   = super::verilog_safe_name( &dfg.name );

    let _n_inputs  = dfg.inputs.len();
    let _n_outputs = dfg.outputs.len();
    let addr_bits = 6u32; /* enough for 16 registers */

    writeln!( out, "// c2v CSR block for {name}" ).unwrap();
    writeln!( out, "// Register map:" ).unwrap();
    writeln!( out, "//   0x00 CTRL    [W] bit0=start" ).unwrap();
    writeln!( out, "//   0x04 STATUS  [R] bit0=done bit1=busy" ).unwrap();
    for (i, port) in dfg.inputs.iter().enumerate() {
        writeln!( out, "//   0x{:02X} INPUT_{} [W] {} ({}b)",
            0x08 + i * 4, i, port.name, port.width ).unwrap();
    }
    for (i, port) in dfg.outputs.iter().enumerate() {
        writeln!( out, "//   0x{:02X} OUTPUT_{} [R] {} ({}b)",
            0x40 + i * 4, i, port.name, port.width ).unwrap();
    }
    writeln!( out ).unwrap();

    writeln!( out, "module {name}_csr (" ).unwrap();
    writeln!( out, "    input  wire              clk," ).unwrap();
    writeln!( out, "    input  wire              rst," ).unwrap();
    writeln!( out ).unwrap();
    writeln!( out, "    /* Register interface */" ).unwrap();
    writeln!( out, "    input  wire [{a}:0] addr,", a = addr_bits - 1 ).unwrap();
    writeln!( out, "    input  wire [31:0] wdata," ).unwrap();
    writeln!( out, "    output reg  [31:0] rdata," ).unwrap();
    writeln!( out, "    input  wire              wen," ).unwrap();
    writeln!( out, "    input  wire              ren" ).unwrap();
    writeln!( out, ");" ).unwrap();
    writeln!( out ).unwrap();

    /* Core signals */
    writeln!( out, "    reg  core_start;" ).unwrap();
    writeln!( out, "    wire core_done;" ).unwrap();
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

    /* Core instance */
    writeln!( out, "    {name} core (" ).unwrap();
    writeln!( out, "        .clk(clk), .rst(rst)," ).unwrap();
    writeln!( out, "        .start(core_start), .done(core_done)," ).unwrap();
    let all_ports: Vec<_> = dfg.inputs.iter().chain( dfg.outputs.iter() ).collect();
    for (i, port) in all_ports.iter().enumerate() {
        let n     = super::verilog_safe_name( &port.name );
        let comma = if i == all_ports.len() - 1 { "" } else { "," };
        let sfx   = match port.direction {
            crate::ir::Direction::In  => "_reg",
            crate::ir::Direction::Out => "_wire",
        };
        writeln!( out, "        .{n}({n}{sfx}){comma}" ).unwrap();
    }
    writeln!( out, "    );" ).unwrap();
    writeln!( out ).unwrap();

    /* Write logic */
    writeln!( out, "    always @(posedge clk) begin" ).unwrap();
    writeln!( out, "        if (rst) begin" ).unwrap();
    writeln!( out, "            core_start <= 1'b0;" ).unwrap();
    for port in &dfg.inputs {
        let n = super::verilog_safe_name( &port.name );
        writeln!( out, "            {n}_reg <= 0;" ).unwrap();
    }
    writeln!( out, "        end else begin" ).unwrap();
    writeln!( out, "            core_start <= 1'b0;" ).unwrap();
    writeln!( out, "            if (wen) begin" ).unwrap();
    writeln!( out, "                case (addr)" ).unwrap();
    writeln!( out, "                    6'h00: core_start <= wdata[0];" ).unwrap();
    for (i, port) in dfg.inputs.iter().enumerate() {
        let n    = super::verilog_safe_name( &port.name );
        let addr = 0x08 / 4 + i;
        writeln!( out, "                    6'h{:02x}: {n}_reg <= wdata[{}:0];",
            addr, port.width.min(32) - 1 ).unwrap();
    }
    writeln!( out, "                    default: ;" ).unwrap();
    writeln!( out, "                endcase" ).unwrap();
    writeln!( out, "            end" ).unwrap();
    writeln!( out, "        end" ).unwrap();
    writeln!( out, "    end" ).unwrap();
    writeln!( out ).unwrap();

    /* Read logic */
    writeln!( out, "    always @(*) begin" ).unwrap();
    writeln!( out, "        rdata = 32'b0;" ).unwrap();
    writeln!( out, "        if (ren) begin" ).unwrap();
    writeln!( out, "            case (addr)" ).unwrap();
    writeln!( out, "                6'h01: rdata = {{30'b0, 1'b0, core_done}};" ).unwrap();
    for (i, port) in dfg.outputs.iter().enumerate() {
        let n    = super::verilog_safe_name( &port.name );
        let addr = 0x40 / 4 + i;
        if port.width < 32 {
            writeln!( out, "                6'h{:02x}: rdata = {{{}'b0, {n}_wire}};",
                addr, 32 - port.width ).unwrap();
        } else {
            writeln!( out, "                6'h{:02x}: rdata = {n}_wire;", addr ).unwrap();
        }
    }
    writeln!( out, "                default: ;" ).unwrap();
    writeln!( out, "            endcase" ).unwrap();
    writeln!( out, "        end" ).unwrap();
    writeln!( out, "    end" ).unwrap();
    writeln!( out ).unwrap();

    writeln!( out, "endmodule" ).unwrap();
    out
}
