/* c2v_rom_infer detects constant arrays (initialized but never
   written after initialization) and marks them as ROM. ROM arrays
   can be synthesized as LUT-based lookup tables or BRAM with
   initialization, rather than requiring runtime store operations.
   This saves cycles (no store phase) and can save area. */


use super::{Dfg, DfgOp, NodeId};

pub struct RomCandidate {
    pub name:      String,
    pub size:      usize,
    pub width:     u32,
    pub values:    Vec<u64>,
    pub n_reads:   usize,
}

/* c2v_find_rom_arrays scans the DFG for arrays that are only
   initialized with constants and never written by dynamic stores.
   Returns a list of ROM candidates with their constant values. */
pub fn find_rom_arrays( dfg: &Dfg ) -> Vec<RomCandidate> {
    let mut candidates = Vec::new();

    for arr in &dfg.arrays {
        /* Collect all stores to this array */
        let stores: Vec<&super::DfgNode> = dfg.nodes.iter()
            .filter(|n| matches!(&n.op, DfgOp::Store { array } if array == &arr.name))
            .collect();

        /* Check if ALL stores write constant values at constant indices */
        let mut is_rom    = true;
        let mut values    = vec![0u64; arr.size];
        let mut has_store = false;

        for store in &stores {
            has_store = true;
            let inputs: Vec<NodeId> = dfg.edges.iter()
                .filter(|e| e.to == store.id && e.width > 0)
                .map(|e| e.from)
                .collect();

            if inputs.len() < 2 {
                is_rom = false;
                break;
            }

            let idx_node = &dfg.nodes[inputs[0] as usize];
            let val_node = &dfg.nodes[inputs[1] as usize];

            match (&idx_node.op, &val_node.op) {
                (DfgOp::Const(idx), DfgOp::Const(val)) => {
                    let i = *idx as usize;
                    if i < arr.size {
                        values[i] = *val;
                    } else {
                        is_rom = false;
                    }
                }
                _ => {
                    /* Non-constant index or value — not a ROM */
                    is_rom = false;
                }
            }

            if !is_rom { break; }
        }

        if !is_rom || !has_store { continue; }

        /* Count reads */
        let n_reads = dfg.nodes.iter()
            .filter(|n| matches!(&n.op, DfgOp::Load { array } if array == &arr.name))
            .count();

        candidates.push(RomCandidate {
            name:   arr.name.clone(),
            size:   arr.size,
            width:  arr.elem_width,
            values,
            n_reads,
        });
    }

    candidates
}

/* c2v_emit_rom_init generates a Verilog initial block or parameter
   array for ROM initialization. For small ROMs (<=16 entries),
   emit as a case statement (LUT). For larger, emit as reg array
   with initial block. */
pub fn emit_rom_init( rom: &RomCandidate, indent: &str ) -> String {
    let mut out = String::with_capacity( 1024 );

    if rom.size <= 16 {
        /* Small ROM: emit as combinational case lookup */
        use std::fmt::Write;
        writeln!(out, "{indent}/* ROM: {} ({} x {}b) */", rom.name, rom.size, rom.width).unwrap();
        writeln!(out, "{indent}function [{w}:0] rom_{name}(input [{aw}:0] addr);",
            w = rom.width - 1,
            name = rom.name,
            aw = ((rom.size as f64).log2().ceil() as u32).max(1) - 1
        ).unwrap();
        writeln!(out, "{indent}    case (addr)").unwrap();
        for (i, val) in rom.values.iter().enumerate() {
            writeln!(out, "{indent}        {aw}'d{i}: rom_{name} = {w}'d{val};",
                aw = (rom.size as f64).log2().ceil() as u32,
                name = rom.name,
                w = rom.width,
            ).unwrap();
        }
        writeln!(out, "{indent}        default: rom_{name} = 0;", name = rom.name).unwrap();
        writeln!(out, "{indent}    endcase").unwrap();
        writeln!(out, "{indent}endfunction").unwrap();
    } else {
        /* Large ROM: emit as initialized reg array */
        use std::fmt::Write;
        writeln!(out, "{indent}/* ROM: {} ({} x {}b) */", rom.name, rom.size, rom.width).unwrap();
        writeln!(out, "{indent}reg [{w}:0] rom_{name} [0:{s}];",
            w = rom.width - 1, name = rom.name, s = rom.size - 1).unwrap();
        writeln!(out, "{indent}initial begin").unwrap();
        for (i, val) in rom.values.iter().enumerate() {
            writeln!(out, "{indent}    rom_{name}[{i}] = {w}'d{val};",
                name = rom.name, w = rom.width).unwrap();
        }
        writeln!(out, "{indent}end").unwrap();
    }

    out
}
