/* c2v_pragma generates synthesis tool attributes/pragmas that guide
   vendor synthesis tools to produce better results. Each vendor has
   different attribute syntax:
   - Xilinx/Vivado: (* attr = "value" *)
   - Intel/Quartus: /* synthesis attr */
   - Lattice/Synplify: /* syn_* */

   These attributes control FSM encoding, RAM style, register
   duplication, and other synthesis decisions. */

#[derive(Debug, Clone, Copy)]
pub enum Vendor {
    Xilinx,
    Intel,
    Lattice,
    Generic,
}

/* c2v_emit_fsm_pragma returns the synthesis attribute for FSM encoding. */
pub fn fsm_encoding_pragma(vendor: Vendor, encoding: &str) -> String {
    match vendor {
        Vendor::Xilinx  => format!("(* fsm_encoding = \"{}\" *)", encoding),
        Vendor::Intel   => format!("/* synthesis syn_encoding = \"{}\" */", encoding),
        Vendor::Lattice => format!("/* syn_encoding = \"{}\" */", encoding),
        Vendor::Generic => String::new(),
    }
}

/* c2v_emit_ram_pragma returns the attribute to force RAM implementation style. */
pub fn ram_style_pragma(vendor: Vendor, style: &str) -> String {
    match vendor {
        Vendor::Xilinx  => format!("(* ram_style = \"{}\" *)", style),
        Vendor::Intel   => format!("/* ramstyle = \"{}\" */", style),
        Vendor::Lattice => format!("/* syn_ramstyle = \"{}\" */", style),
        Vendor::Generic => String::new(),
    }
}

/* c2v_emit_keep_pragma prevents the synthesizer from optimizing away
   a signal. Useful for debug probes and pipeline registers. */
pub fn keep_pragma(vendor: Vendor) -> String {
    match vendor {
        Vendor::Xilinx  => "(* KEEP = \"TRUE\" *)".to_string(),
        Vendor::Intel   => "/* synthesis preserve */".to_string(),
        Vendor::Lattice => "/* syn_keep = 1 */".to_string(),
        Vendor::Generic => String::new(),
    }
}

/* c2v_emit_max_fanout_pragma limits fanout to improve timing. */
pub fn max_fanout_pragma(vendor: Vendor, limit: u32) -> String {
    match vendor {
        Vendor::Xilinx  => format!("(* MAX_FANOUT = {} *)", limit),
        Vendor::Intel   => format!("/* synthesis maxfan = {} */", limit),
        Vendor::Lattice => String::new(),
        Vendor::Generic => String::new(),
    }
}

/* c2v_emit_dont_touch prevents any optimization on a signal/module. */
pub fn dont_touch_pragma(vendor: Vendor) -> String {
    match vendor {
        Vendor::Xilinx  => "(* DONT_TOUCH = \"TRUE\" *)".to_string(),
        Vendor::Intel   => "/* synthesis noprune */".to_string(),
        Vendor::Lattice => "/* syn_noprune = 1 */".to_string(),
        Vendor::Generic => String::new(),
    }
}

/* c2v_emit_use_dsp_pragma forces DSP block inference for multiplies. */
pub fn use_dsp_pragma(vendor: Vendor) -> String {
    match vendor {
        Vendor::Xilinx  => "(* use_dsp = \"yes\" *)".to_string(),
        Vendor::Intel   => "/* synthesis multstyle = \"dsp\" */".to_string(),
        Vendor::Lattice => String::new(),
        Vendor::Generic => String::new(),
    }
}
