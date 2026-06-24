/* c2v_ila_probes generates Xilinx ILA (Integrated Logic Analyzer)
   debug probe instantiation for on-chip debugging. The user can
   mark signals of interest and c2v generates the ILA core
   instantiation with proper probe widths and trigger conditions.

   This lets you observe the c2v-generated hardware running on a
   real FPGA: see when the FSM transitions, watch data flow through
   pipeline stages, and verify timing in silicon. */

use std::fmt::Write;

use crate::ir::BoundDfg;

#[derive(Debug, Clone)]
pub struct ProbeConfig {
    pub name:   String,
    pub width:  u32,
    pub is_trigger: bool,
}

/* c2v_emit_ila generates a Vivado ILA debug core instantiation
   that monitors key signals in the c2v module. Automatically
   adds probes for: FSM state, done, all input ports, all output
   ports, and optionally internal registers. */
pub fn emit_ila_instantiation(
    bound:      &BoundDfg,
    ila_depth:  u32,
) -> String {
    let mut out = String::with_capacity( 2048 );
    let dfg  = &bound.scheduled.dfg;
    let name = super::verilog_safe_name( &dfg.name );

    /* Collect probes */
    let mut probes: Vec<ProbeConfig> = Vec::new();

    /* FSM state */
    let state_bits = super::state_width( bound.scheduled.total_cycles );
    probes.push( ProbeConfig {
        name:       "state".to_string(),
        width:      state_bits,
        is_trigger: true,
    });

    /* Control signals */
    probes.push( ProbeConfig { name: "start".into(), width: 1, is_trigger: true });
    probes.push( ProbeConfig { name: "done".into(),  width: 1, is_trigger: true });

    /* Input ports */
    for port in &dfg.inputs {
        probes.push( ProbeConfig {
            name:       super::verilog_safe_name( &port.name ),
            width:      port.width,
            is_trigger: false,
        });
    }

    /* Output ports */
    for port in &dfg.outputs {
        probes.push( ProbeConfig {
            name:       super::verilog_safe_name( &port.name ),
            width:      port.width,
            is_trigger: false,
        });
    }

    writeln!( out, "/* c2v ILA debug core for {name} */" ).unwrap();
    writeln!( out, "/* Probes: {} signals, depth: {} samples */",
        probes.len(), ila_depth ).unwrap();
    writeln!( out ).unwrap();

    writeln!( out, "(* DONT_TOUCH = \"TRUE\" *)" ).unwrap();
    writeln!( out, "ila_0 c2v_ila (" ).unwrap();
    writeln!( out, "    .clk( clk )," ).unwrap();

    for (i, probe) in probes.iter().enumerate() {
        let comma = if i == probes.len() - 1 { "" } else { "," };
        if probe.width == 1 {
            writeln!( out, "    .probe{i}( {} ){comma}", probe.name ).unwrap();
        } else {
            writeln!( out, "    .probe{i}( {} ){comma}  /* [{w}:0] */",
                probe.name, w = probe.width - 1 ).unwrap();
        }
    }
    writeln!( out, ");" ).unwrap();
    writeln!( out ).unwrap();

    /* Generate TCL for ILA creation */
    writeln!( out, "/* Vivado TCL to create this ILA:" ).unwrap();
    writeln!( out, "   create_debug_core u_ila_0 ila" ).unwrap();
    writeln!( out, "   set_property C_DATA_DEPTH {ila_depth} [get_debug_cores u_ila_0]" ).unwrap();
    writeln!( out, "   set_property C_NUM_OF_PROBES {} [get_debug_cores u_ila_0]",
        probes.len() ).unwrap();
    for (i, probe) in probes.iter().enumerate() {
        writeln!( out, "   set_property port_width {} [get_debug_ports u_ila_0/probe{i}]",
            probe.width ).unwrap();
        writeln!( out, "   connect_debug_port u_ila_0/probe{i} [get_nets {{{name}/{n}*}}]",
            name = name, n = probe.name ).unwrap();
    }
    writeln!( out, "*/" ).unwrap();

    out
}
