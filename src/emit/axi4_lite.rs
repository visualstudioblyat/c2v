/* c2v_axi4_lite generates an AXI4-Lite slave wrapper that provides
   a register-mapped interface to the c2v module. Unlike CSR (which
   is a simple register bus), AXI4-Lite is the standard Xilinx/ARM
   protocol used by Vivado's address editor, ZYNQ PS→PL bridges,
   and PCIe BARs.

   Register map (same as CSR but with AXI4-Lite handshaking):
     0x00: CTRL    [W]  bit 0 = start
     0x04: STATUS  [R]  bit 0 = done
     0x08+: INPUTS [W]  one 32-bit register per input port
     0x40+: OUTPUTS[R]  one 32-bit register per output port */

use std::fmt::Write;

use crate::ir::BoundDfg;

pub fn emit_axi4_lite_slave( bound: &BoundDfg ) -> String {
    let mut out = String::with_capacity( 8192 );
    let dfg  = &bound.scheduled.dfg;
    let name = super::verilog_safe_name( &dfg.name );
    let addr_bits = 6u32;

    writeln!( out, "// c2v AXI4-Lite slave for {name}" ).unwrap();
    writeln!( out ).unwrap();

    writeln!( out, "module {name}_axilite (" ).unwrap();
    writeln!( out, "    input  wire              S_AXI_ACLK," ).unwrap();
    writeln!( out, "    input  wire              S_AXI_ARESETN," ).unwrap();
    writeln!( out ).unwrap();

    /* Write address channel */
    writeln!( out, "    input  wire [{a}:0] S_AXI_AWADDR,", a = addr_bits - 1 ).unwrap();
    writeln!( out, "    input  wire              S_AXI_AWVALID," ).unwrap();
    writeln!( out, "    output reg               S_AXI_AWREADY," ).unwrap();

    /* Write data channel */
    writeln!( out, "    input  wire [31:0] S_AXI_WDATA," ).unwrap();
    writeln!( out, "    input  wire [ 3:0] S_AXI_WSTRB," ).unwrap();
    writeln!( out, "    input  wire              S_AXI_WVALID," ).unwrap();
    writeln!( out, "    output reg               S_AXI_WREADY," ).unwrap();

    /* Write response */
    writeln!( out, "    output reg  [ 1:0] S_AXI_BRESP," ).unwrap();
    writeln!( out, "    output reg               S_AXI_BVALID," ).unwrap();
    writeln!( out, "    input  wire              S_AXI_BREADY," ).unwrap();

    /* Read address channel */
    writeln!( out, "    input  wire [{a}:0] S_AXI_ARADDR,", a = addr_bits - 1 ).unwrap();
    writeln!( out, "    input  wire              S_AXI_ARVALID," ).unwrap();
    writeln!( out, "    output reg               S_AXI_ARREADY," ).unwrap();

    /* Read data channel */
    writeln!( out, "    output reg  [31:0] S_AXI_RDATA," ).unwrap();
    writeln!( out, "    output reg  [ 1:0] S_AXI_RRESP," ).unwrap();
    writeln!( out, "    output reg               S_AXI_RVALID," ).unwrap();
    writeln!( out, "    input  wire              S_AXI_RREADY" ).unwrap();
    writeln!( out, ");" ).unwrap();
    writeln!( out ).unwrap();

    writeln!( out, "    wire rst = ~S_AXI_ARESETN;" ).unwrap();
    writeln!( out, "    reg  core_start;" ).unwrap();
    writeln!( out, "    wire core_done;" ).unwrap();
    writeln!( out ).unwrap();

    /* Input/output registers */
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

    /* Core instantiation */
    writeln!( out, "    {name} core (" ).unwrap();
    writeln!( out, "        .clk(S_AXI_ACLK), .rst(rst)," ).unwrap();
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

    /* AXI4-Lite write logic */
    writeln!( out, "    reg [{a}:0] aw_addr;", a = addr_bits - 1 ).unwrap();
    writeln!( out ).unwrap();

    writeln!( out, "    always @(posedge S_AXI_ACLK) begin" ).unwrap();
    writeln!( out, "        if (rst) begin" ).unwrap();
    writeln!( out, "            S_AXI_AWREADY <= 1'b0;" ).unwrap();
    writeln!( out, "            S_AXI_WREADY  <= 1'b0;" ).unwrap();
    writeln!( out, "            S_AXI_BVALID  <= 1'b0;" ).unwrap();
    writeln!( out, "            S_AXI_BRESP   <= 2'b0;" ).unwrap();
    writeln!( out, "            core_start    <= 1'b0;" ).unwrap();
    for port in &dfg.inputs {
        let n = super::verilog_safe_name( &port.name );
        writeln!( out, "            {n}_reg <= 0;" ).unwrap();
    }
    writeln!( out, "        end else begin" ).unwrap();
    writeln!( out, "            core_start <= 1'b0;" ).unwrap();

    /* Write address accept */
    writeln!( out, "            if (S_AXI_AWVALID && !S_AXI_AWREADY) begin" ).unwrap();
    writeln!( out, "                S_AXI_AWREADY <= 1'b1;" ).unwrap();
    writeln!( out, "                aw_addr       <= S_AXI_AWADDR;" ).unwrap();
    writeln!( out, "            end else S_AXI_AWREADY <= 1'b0;" ).unwrap();

    /* Write data accept + register update */
    writeln!( out, "            if (S_AXI_WVALID && !S_AXI_WREADY) begin" ).unwrap();
    writeln!( out, "                S_AXI_WREADY <= 1'b1;" ).unwrap();
    writeln!( out, "                case (aw_addr[{a}:2])", a = addr_bits - 1 ).unwrap();
    writeln!( out, "                    0: core_start <= S_AXI_WDATA[0];" ).unwrap();
    for (i, port) in dfg.inputs.iter().enumerate() {
        let n = super::verilog_safe_name( &port.name );
        writeln!( out, "                    {}: {n}_reg <= S_AXI_WDATA[{}:0];",
            i + 2, port.width.min(32) - 1 ).unwrap();
    }
    writeln!( out, "                    default: ;" ).unwrap();
    writeln!( out, "                endcase" ).unwrap();
    writeln!( out, "            end else S_AXI_WREADY <= 1'b0;" ).unwrap();

    /* Write response */
    writeln!( out, "            if (S_AXI_BREADY && S_AXI_BVALID)" ).unwrap();
    writeln!( out, "                S_AXI_BVALID <= 1'b0;" ).unwrap();
    writeln!( out, "            else if (S_AXI_AWREADY)" ).unwrap();
    writeln!( out, "                S_AXI_BVALID <= 1'b1;" ).unwrap();

    writeln!( out, "        end" ).unwrap();
    writeln!( out, "    end" ).unwrap();
    writeln!( out ).unwrap();

    /* AXI4-Lite read logic */
    writeln!( out, "    always @(posedge S_AXI_ACLK) begin" ).unwrap();
    writeln!( out, "        if (rst) begin" ).unwrap();
    writeln!( out, "            S_AXI_ARREADY <= 1'b0;" ).unwrap();
    writeln!( out, "            S_AXI_RVALID  <= 1'b0;" ).unwrap();
    writeln!( out, "            S_AXI_RRESP   <= 2'b0;" ).unwrap();
    writeln!( out, "            S_AXI_RDATA   <= 0;" ).unwrap();
    writeln!( out, "        end else begin" ).unwrap();
    writeln!( out, "            if (S_AXI_ARVALID && !S_AXI_ARREADY) begin" ).unwrap();
    writeln!( out, "                S_AXI_ARREADY <= 1'b1;" ).unwrap();
    writeln!( out, "                case (S_AXI_ARADDR[{a}:2])", a = addr_bits - 1 ).unwrap();
    writeln!( out, "                    1: S_AXI_RDATA <= {{31'b0, core_done}};" ).unwrap();
    for (i, port) in dfg.outputs.iter().enumerate() {
        let n = super::verilog_safe_name( &port.name );
        let addr = 0x40 / 4 + i;
        if port.width < 32 {
            writeln!( out, "                    {addr}: S_AXI_RDATA <= {{{}'b0, {n}_wire}};",
                32 - port.width ).unwrap();
        } else {
            writeln!( out, "                    {addr}: S_AXI_RDATA <= {n}_wire;" ).unwrap();
        }
    }
    writeln!( out, "                    default: S_AXI_RDATA <= 0;" ).unwrap();
    writeln!( out, "                endcase" ).unwrap();
    writeln!( out, "                S_AXI_RVALID <= 1'b1;" ).unwrap();
    writeln!( out, "            end else begin" ).unwrap();
    writeln!( out, "                S_AXI_ARREADY <= 1'b0;" ).unwrap();
    writeln!( out, "                if (S_AXI_RREADY && S_AXI_RVALID)" ).unwrap();
    writeln!( out, "                    S_AXI_RVALID <= 1'b0;" ).unwrap();
    writeln!( out, "            end" ).unwrap();
    writeln!( out, "        end" ).unwrap();
    writeln!( out, "    end" ).unwrap();
    writeln!( out ).unwrap();

    writeln!( out, "endmodule" ).unwrap();
    out
}
