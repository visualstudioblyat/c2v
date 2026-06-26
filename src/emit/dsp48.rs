/* c2v_dsp48 emits DSP48E1/E2 primitive instantiations for Xilinx.
   Instead of generic `a * b + c` using fabric multipliers, we emit:

     DSP48E1 #(.USE_MULT("MULTIPLY"), .MREG(1)) dsp_inst (
       .A(a), .B(b), .C(c), .P(result), ...
     );

   This gives:
   - 25×18 unsigned multiply (or 24×17 signed)
   - Optional pre-adder: (A ± D) × B
   - Optional post-adder: M + C or M + P (accumulate)
   - Deterministic 3-cycle latency
   - No fabric LUT usage for the multiply */

use std::fmt::Write;

use crate::ir::dsp_infer::{DspMapping, DspPattern};

pub fn emit_dsp48e1( mapping: &DspMapping, idx: usize ) -> String {
    let mut out = String::with_capacity( 2048 );

    match &mapping.pattern {
        DspPattern::Multiply { a, b } => {
            writeln!( out, "    // DSP48E1 multiply: node {}", mapping.mul_node ).unwrap();
            writeln!( out, "    wire [47:0] dsp_{idx}_p;" ).unwrap();
            writeln!( out, "    DSP48E1 #(" ).unwrap();
            writeln!( out, "        .USE_MULT(\"MULTIPLY\")," ).unwrap();
            writeln!( out, "        .AREG(1), .BREG(1), .MREG(1)" ).unwrap();
            writeln!( out, "    ) dsp_{idx} (" ).unwrap();
            writeln!( out, "        .CLK(clk)," ).unwrap();
            writeln!( out, "        .CEA2(1'b1), .CEB2(1'b1), .CEM(1'b1)," ).unwrap();
            writeln!( out, "        .RSTA(rst), .RSTB(rst), .RSTM(rst)," ).unwrap();
            writeln!( out, "        .A({{{{5{{1'b0}}}}, node_{a}[{aw}:0]}})," , aw = mapping.a_width - 1 ).unwrap();
            writeln!( out, "        .B({{{{0}}, node_{b}[{bw}:0]}})," , bw = mapping.b_width - 1 ).unwrap();
            writeln!( out, "        .C(48'b0)," ).unwrap();
            writeln!( out, "        .OPMODE(7'b0000101)," ).unwrap();
            writeln!( out, "        .P(dsp_{idx}_p)" ).unwrap();
            writeln!( out, "    );" ).unwrap();
        }
        DspPattern::MultiplyAccumulate { a, b, c } => {
            writeln!( out, "    // DSP48E1 MAC: node {} + {}", mapping.mul_node,
                mapping.add_node.unwrap_or(0) ).unwrap();
            writeln!( out, "    wire [47:0] dsp_{idx}_p;" ).unwrap();
            writeln!( out, "    DSP48E1 #(" ).unwrap();
            writeln!( out, "        .USE_MULT(\"MULTIPLY\")," ).unwrap();
            writeln!( out, "        .AREG(1), .BREG(1), .CREG(1), .MREG(1), .PREG(1)" ).unwrap();
            writeln!( out, "    ) dsp_{idx} (" ).unwrap();
            writeln!( out, "        .CLK(clk)," ).unwrap();
            writeln!( out, "        .CEA2(1'b1), .CEB2(1'b1), .CEC(1'b1), .CEM(1'b1), .CEP(1'b1)," ).unwrap();
            writeln!( out, "        .RSTA(rst), .RSTB(rst), .RSTC(rst), .RSTM(rst), .RSTP(rst)," ).unwrap();
            writeln!( out, "        .A({{{{5{{1'b0}}}}, node_{a}[{aw}:0]}})," , aw = mapping.a_width - 1 ).unwrap();
            writeln!( out, "        .B({{{{0}}, node_{b}[{bw}:0]}})," , bw = mapping.b_width - 1 ).unwrap();
            writeln!( out, "        .C({{{{16{{1'b0}}}}, node_{c}}})," ).unwrap();
            writeln!( out, "        .OPMODE(7'b0110101)," ).unwrap();
            writeln!( out, "        .P(dsp_{idx}_p)" ).unwrap();
            writeln!( out, "    );" ).unwrap();
        }
        DspPattern::PreAddMultiply { a, d, b } => {
            writeln!( out, "    // DSP48E1 pre-add multiply: (node_{a} + node_{d}) * node_{b}" ).unwrap();
            writeln!( out, "    wire [47:0] dsp_{idx}_p;" ).unwrap();
            writeln!( out, "    DSP48E1 #(" ).unwrap();
            writeln!( out, "        .USE_MULT(\"MULTIPLY\")," ).unwrap();
            writeln!( out, "        .USE_DPORT(\"TRUE\")," ).unwrap();
            writeln!( out, "        .ADREG(1), .AREG(1), .BREG(1), .DREG(1), .MREG(1)" ).unwrap();
            writeln!( out, "    ) dsp_{idx} (" ).unwrap();
            writeln!( out, "        .CLK(clk)," ).unwrap();
            writeln!( out, "        .CEA2(1'b1), .CEB2(1'b1), .CED(1'b1), .CEAD(1'b1), .CEM(1'b1)," ).unwrap();
            writeln!( out, "        .RSTA(rst), .RSTB(rst), .RSTD(rst), .RSTM(rst)," ).unwrap();
            writeln!( out, "        .A({{{{5{{1'b0}}}}, node_{a}}})," ).unwrap();
            writeln!( out, "        .B({{{{0}}, node_{b}}})," ).unwrap();
            writeln!( out, "        .D({{{{7{{1'b0}}}}, node_{d}}})," ).unwrap();
            writeln!( out, "        .INMODE(5'b00100)," ).unwrap();
            writeln!( out, "        .OPMODE(7'b0000101)," ).unwrap();
            writeln!( out, "        .P(dsp_{idx}_p)" ).unwrap();
            writeln!( out, "    );" ).unwrap();
        }
    }

    out
}

pub fn emit_all_dsp48( mappings: &[DspMapping] ) -> String {
    let mut out = String::new();
    if mappings.is_empty() { return out; }

    writeln!( out, "    // DSP48E1 inferred instances" ).unwrap();
    for (i, m) in mappings.iter().enumerate() {
        write!( out, "{}", emit_dsp48e1( m, i ) ).unwrap();
        writeln!( out ).unwrap();
    }
    out
}
