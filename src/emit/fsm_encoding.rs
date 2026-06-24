/* c2v_fsm_encoding provides different FSM state encoding strategies.
   The default is binary encoding (sequential numbers). One-hot encoding
   uses one flip-flop per state, which is faster on FPGAs because the
   next-state logic is simpler (one LUT per transition vs decoder). */

use std::fmt::Write;

#[derive(Debug, Clone, Copy)]
pub enum FsmEncoding {
    Binary,
    OneHot,
    Gray,
}

/* c2v_emit_fsm_params generates the state parameter declarations
   for the given encoding and number of states. */
pub fn emit_state_params(
    total_cycles: u32,
    encoding:     FsmEncoding,
) -> (String, String, u32) {
    let n_states  = total_cycles + 2; /* IDLE + cycles + DONE */
    let mut out   = String::with_capacity( 1024 );

    match encoding {
        FsmEncoding::Binary => {
            let bits = (32 - n_states.leading_zeros()).max(1);
            writeln!( out, "    localparam [{b}:0] S_IDLE = {bits}'d0;", b = bits - 1 ).unwrap();
            for c in 0..total_cycles {
                writeln!( out, "    localparam [{b}:0] S_C{c} = {bits}'d{v};",
                    b = bits - 1, v = c + 1 ).unwrap();
            }
            writeln!( out, "    localparam [{b}:0] S_DONE = {bits}'d{v};",
                b = bits - 1, v = total_cycles + 1 ).unwrap();
            let state_decl = format!( "    reg [{b}:0] state;", b = bits - 1 );
            (out, state_decl, bits)
        }
        FsmEncoding::OneHot => {
            let bits = n_states;
            writeln!( out, "    localparam [{b}:0] S_IDLE = {bits}'b{v};",
                b = bits - 1, v = one_hot_str(0, bits) ).unwrap();
            for c in 0..total_cycles {
                writeln!( out, "    localparam [{b}:0] S_C{c} = {bits}'b{v};",
                    b = bits - 1, v = one_hot_str(c + 1, bits) ).unwrap();
            }
            writeln!( out, "    localparam [{b}:0] S_DONE = {bits}'b{v};",
                b = bits - 1, v = one_hot_str(total_cycles + 1, bits) ).unwrap();
            let state_decl = format!(
                "    (* fsm_encoding = \"one_hot\" *) reg [{b}:0] state;",
                b = bits - 1 );
            (out, state_decl, bits)
        }
        FsmEncoding::Gray => {
            let bits = (32 - n_states.leading_zeros()).max(1);
            writeln!( out, "    localparam [{b}:0] S_IDLE = {bits}'d0;", b = bits - 1 ).unwrap();
            for c in 0..total_cycles {
                let v = (c + 1) ^ ((c + 1) >> 1);
                writeln!( out, "    localparam [{b}:0] S_C{c} = {bits}'d{v};",
                    b = bits - 1 ).unwrap();
            }
            let v_done = (total_cycles + 1) ^ ((total_cycles + 1) >> 1);
            writeln!( out, "    localparam [{b}:0] S_DONE = {bits}'d{v_done};",
                b = bits - 1 ).unwrap();
            let state_decl = format!( "    reg [{b}:0] state;", b = bits - 1 );
            (out, state_decl, bits)
        }
    }
}

fn one_hot_str( bit: u32, width: u32 ) -> String {
    let mut s = String::with_capacity( width as usize );
    for i in (0..width).rev() {
        if i == bit { s.push('1'); } else { s.push('0'); }
    }
    s
}
