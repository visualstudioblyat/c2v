/* c2v_xdc_constraints generates Xilinx Design Constraints (.xdc).
   XDC is Vivado's native format — a superset of SDC with Xilinx-
   specific extensions for pin placement, I/O standards, and
   implementation directives.

   For a standalone c2v IP core on an FPGA board, the XDC maps:
   - Clock pin to the board oscillator
   - I/O pins to the board connector (PMOD, FMC, etc.)
   - I/O voltage standard (LVCMOS33 for PMOD, LVDS for FMC)
   - Implementation strategy (performance vs area) */

use std::fmt::Write;

use crate::ir::BoundDfg;

#[derive(Debug, Clone)]
pub struct XdcConfig {
    pub clock_pin:      String,
    pub clock_period_ns: f64,
    pub io_standard:    String,
    pub board:          BoardPreset,
}

#[derive(Debug, Clone)]
pub enum BoardPreset {
    ArtyA7,
    Basys3,
    ZyboZ7,
    ZCU104,
    Custom,
}

impl Default for XdcConfig {
    fn default() -> Self {
        XdcConfig {
            clock_pin:       "E3".to_string(),
            clock_period_ns: 10.0,
            io_standard:     "LVCMOS33".to_string(),
            board:           BoardPreset::ArtyA7,
        }
    }
}

pub fn emit_xdc( bound: &BoundDfg, config: &XdcConfig ) -> String {
    let mut out = String::with_capacity( 4096 );
    let dfg  = &bound.scheduled.dfg;
    let name = crate::emit::verilog_safe_name( &dfg.name );

    writeln!( out, "## c2v XDC constraints for {name}" ).unwrap();
    writeln!( out, "## Board: {:?}", config.board ).unwrap();
    writeln!( out, "## Target: {:.1} MHz", 1000.0 / config.clock_period_ns ).unwrap();
    writeln!( out ).unwrap();

    /* Clock */
    writeln!( out, "## Clock" ).unwrap();
    writeln!( out, "set_property -dict {{ PACKAGE_PIN {} IOSTANDARD {} }} [get_ports clk]",
        config.clock_pin, config.io_standard ).unwrap();
    writeln!( out, "create_clock -add -name sys_clk -period {:.3} -waveform {{0 {:.3}}} [get_ports clk]",
        config.clock_period_ns, config.clock_period_ns / 2.0 ).unwrap();
    writeln!( out ).unwrap();

    /* Board-specific PMOD pin map */
    let pmod_pins = match config.board {
        BoardPreset::ArtyA7 => vec![
            "G13", "B11", "A11", "D12", "D13", "B18", "A18", "K16",
            "E15", "E16", "D15", "C15", "J17", "J18", "K15", "J15",
        ],
        BoardPreset::Basys3 => vec![
            "J1", "L2", "J2", "G2", "H1", "K2", "H2", "G3",
            "A14", "A16", "B15", "B16", "A15", "A17", "C15", "C16",
        ],
        BoardPreset::ZyboZ7 => vec![
            "Y18", "Y19", "Y16", "Y17", "U18", "U19", "W18", "W19",
            "W14", "Y14", "T11", "T10", "V16", "W16", "V12", "W13",
        ],
        _ => Vec::new(),
    };

    if !pmod_pins.is_empty() {
        writeln!( out, "## I/O pin mapping (PMOD connectors)" ).unwrap();

        let mut pin_idx = 0;

        /* Reset + start */
        if pin_idx < pmod_pins.len() {
            writeln!( out, "set_property -dict {{ PACKAGE_PIN {} IOSTANDARD {} }} [get_ports rst]",
                pmod_pins[pin_idx], config.io_standard ).unwrap();
            pin_idx += 1;
        }
        if pin_idx < pmod_pins.len() {
            writeln!( out, "set_property -dict {{ PACKAGE_PIN {} IOSTANDARD {} }} [get_ports start]",
                pmod_pins[pin_idx], config.io_standard ).unwrap();
            pin_idx += 1;
        }
        if pin_idx < pmod_pins.len() {
            writeln!( out, "set_property -dict {{ PACKAGE_PIN {} IOSTANDARD {} }} [get_ports done]",
                pmod_pins[pin_idx], config.io_standard ).unwrap();
            pin_idx += 1;
        }
        writeln!( out ).unwrap();

        /* Input ports (1-bit each, for simple testing) */
        for port in &dfg.inputs {
            if pin_idx >= pmod_pins.len() { break; }
            let n = crate::emit::verilog_safe_name( &port.name );
            if port.width == 1 {
                writeln!( out, "set_property -dict {{ PACKAGE_PIN {} IOSTANDARD {} }} [get_ports {{{n}}}]",
                    pmod_pins[pin_idx], config.io_standard ).unwrap();
                pin_idx += 1;
            } else {
                writeln!( out, "## {n} [{w}:0] — too wide for PMOD, use AXI instead",
                    w = port.width - 1 ).unwrap();
            }
        }
    }
    writeln!( out ).unwrap();

    /* Implementation strategy */
    writeln!( out, "## Implementation strategy" ).unwrap();
    writeln!( out, "set_property STEPS.SYNTH_DESIGN.ARGS.RETIMING true [get_runs synth_1]" ).unwrap();
    writeln!( out, "set_property STEPS.OPT_DESIGN.ARGS.DIRECTIVE Explore [get_runs impl_1]" ).unwrap();
    writeln!( out, "set_property STEPS.PLACE_DESIGN.ARGS.DIRECTIVE ExtraPostPlacementOpt [get_runs impl_1]" ).unwrap();
    writeln!( out, "set_property STEPS.ROUTE_DESIGN.ARGS.DIRECTIVE AggressiveExplore [get_runs impl_1]" ).unwrap();

    out
}
