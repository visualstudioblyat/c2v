/* c2v_sdc_constraints generates Synopsys Design Constraints (.sdc)
   for timing closure. SDC is the industry-standard format used by
   Vivado, Quartus, and Yosys/nextpnr for timing-driven P&R.

   We generate:
   1. Clock definition with target period
   2. Input delay constraints (when inputs arrive relative to clock)
   3. Output delay constraints (when outputs must be stable)
   4. Multicycle path exceptions for operations that take >1 cycle
   5. False path constraints for reset and test signals
   6. Max delay constraints for critical paths */

use std::fmt::Write;

use crate::ir::BoundDfg;

pub struct SdcConfig {
    pub clock_name:     String,
    pub clock_period_ns: f64,
    pub input_delay_ns:  f64,
    pub output_delay_ns: f64,
}

impl Default for SdcConfig {
    fn default() -> Self {
        SdcConfig {
            clock_name:       "clk".to_string(),
            clock_period_ns:  10.0,
            input_delay_ns:   2.0,
            output_delay_ns:  2.0,
        }
    }
}

pub fn emit_sdc( bound: &BoundDfg, config: &SdcConfig ) -> String {
    let mut out = String::with_capacity( 4096 );
    let dfg  = &bound.scheduled.dfg;
    let name = crate::emit::verilog_safe_name( &dfg.name );

    writeln!( out, "# c2v SDC constraints for {name}" ).unwrap();
    writeln!( out, "# Target: {:.1} MHz ({:.2} ns period)",
        1000.0 / config.clock_period_ns, config.clock_period_ns ).unwrap();
    writeln!( out ).unwrap();

    /* Clock definition */
    writeln!( out, "create_clock -name {} -period {:.3} [get_ports {{{}}}]",
        config.clock_name, config.clock_period_ns, config.clock_name ).unwrap();
    writeln!( out ).unwrap();

    /* Input delays */
    writeln!( out, "# Input timing constraints" ).unwrap();
    writeln!( out, "set_input_delay -clock {} {:.3} [get_ports {{start}}]",
        config.clock_name, config.input_delay_ns ).unwrap();
    for port in &dfg.inputs {
        let n = crate::emit::verilog_safe_name( &port.name );
        if port.width > 1 {
            writeln!( out, "set_input_delay -clock {} {:.3} [get_ports {{{n}[*]}}]",
                config.clock_name, config.input_delay_ns ).unwrap();
        } else {
            writeln!( out, "set_input_delay -clock {} {:.3} [get_ports {{{n}}}]",
                config.clock_name, config.input_delay_ns ).unwrap();
        }
    }
    writeln!( out ).unwrap();

    /* Output delays */
    writeln!( out, "# Output timing constraints" ).unwrap();
    writeln!( out, "set_output_delay -clock {} {:.3} [get_ports {{done}}]",
        config.clock_name, config.output_delay_ns ).unwrap();
    for port in &dfg.outputs {
        let n = crate::emit::verilog_safe_name( &port.name );
        if port.width > 1 {
            writeln!( out, "set_output_delay -clock {} {:.3} [get_ports {{{n}[*]}}]",
                config.clock_name, config.output_delay_ns ).unwrap();
        } else {
            writeln!( out, "set_output_delay -clock {} {:.3} [get_ports {{{n}}}]",
                config.clock_name, config.output_delay_ns ).unwrap();
        }
    }
    writeln!( out ).unwrap();

    /* False paths on reset */
    writeln!( out, "# Reset is asynchronous — exclude from timing" ).unwrap();
    writeln!( out, "set_false_path -from [get_ports {{rst}}]" ).unwrap();
    writeln!( out ).unwrap();

    /* Multicycle paths for multiply (if DSP is pipelined) */
    let has_mul = dfg.nodes.iter().any(|n| matches!(&n.op, crate::ir::DfgOp::Mul));
    if has_mul {
        writeln!( out, "# Multicycle exception for pipelined DSP multipliers" ).unwrap();
        writeln!( out, "# Uncomment if DSP48 is configured for 2-cycle latency:" ).unwrap();
        writeln!( out, "# set_multicycle_path 2 -setup -from [get_cells {{*mul*}}]" ).unwrap();
        writeln!( out, "# set_multicycle_path 1 -hold  -from [get_cells {{*mul*}}]" ).unwrap();
        writeln!( out ).unwrap();
    }

    /* Max delay for critical path */
    writeln!( out, "# Design-wide constraints" ).unwrap();
    let half_period = config.clock_period_ns / 2.0;
    writeln!( out, "set_max_delay {:.3} -from [all_inputs] -to [all_outputs]",
        config.clock_period_ns * bound.scheduled.total_cycles as f64 ).unwrap();
    writeln!( out, "set_max_fanout 32 [current_design]" ).unwrap();
    writeln!( out, "set_max_transition {:.3} [current_design]", half_period ).unwrap();
    writeln!( out ).unwrap();

    /* Clock uncertainty */
    writeln!( out, "# Clock uncertainty (jitter + skew)" ).unwrap();
    writeln!( out, "set_clock_uncertainty 0.100 [get_clocks {{{}}}]",
        config.clock_name ).unwrap();

    out
}
