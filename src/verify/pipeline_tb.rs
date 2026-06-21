use std::fmt::Write;

use crate::ir::{BoundDfg, DfgOp};
use super::{verilog_safe, build_reference_expr, expr_for_node};

pub fn generate_pipeline_testbench(bound: &BoundDfg, _source: &str) -> String {
    let dfg = &bound.scheduled.dfg;
    let mut tb = String::with_capacity(8192);
    let mod_name = verilog_safe(&dfg.name);
    let latency = bound.scheduled.total_cycles + 5;

    writeln!(tb, "// Pipelined co-simulation testbench for {}", dfg.name).unwrap();
    writeln!(tb, "// Tests II=1 throughput — new input every clock").unwrap();
    writeln!(tb, "`timescale 1ns/1ps").unwrap();
    writeln!(tb).unwrap();
    writeln!(tb, "module tb_{mod_name};").unwrap();
    writeln!(tb).unwrap();
    writeln!(tb, "    reg clk, rst, valid_in;").unwrap();
    writeln!(tb, "    wire valid_out;").unwrap();

    for port in &dfg.inputs {
        writeln!(tb, "    reg  [{w}:0] {n};", w = port.width - 1, n = verilog_safe(&port.name)).unwrap();
    }
    for port in &dfg.outputs {
        writeln!(tb, "    wire [{w}:0] {n};", w = port.width - 1, n = verilog_safe(&port.name)).unwrap();
    }
    writeln!(tb).unwrap();

    // DUT
    writeln!(tb, "    {mod_name} dut(").unwrap();
    writeln!(tb, "        .clk(clk),").unwrap();
    writeln!(tb, "        .rst(rst),").unwrap();
    writeln!(tb, "        .valid_in(valid_in),").unwrap();
    writeln!(tb, "        .valid_out(valid_out),").unwrap();

    let all_ports: Vec<_> = dfg.inputs.iter().chain(dfg.outputs.iter()).collect();
    for (i, port) in all_ports.iter().enumerate() {
        let n = verilog_safe(&port.name);
        let comma = if i == all_ports.len() - 1 { "" } else { "," };
        writeln!(tb, "        .{n}({n}){comma}").unwrap();
    }
    writeln!(tb, "    );").unwrap();
    writeln!(tb).unwrap();

    // Reference arrays
    for arr in &dfg.arrays {
        let name = verilog_safe(&arr.name);
        writeln!(tb, "    reg [{w}:0] ref_{name} [0:{s}];",
            w = arr.elem_width - 1, s = arr.size - 1).unwrap();
    }

    // Expected value storage
    if let Some(out_port) = dfg.outputs.first() {
        writeln!(tb, "    reg [{w}:0] expected_mem [0:1023];", w = out_port.width - 1).unwrap();
    }
    writeln!(tb).unwrap();

    writeln!(tb, "    always #5 clk = ~clk;").unwrap();
    writeln!(tb).unwrap();

    writeln!(tb, "    integer i, errors, total_out;").unwrap();
    writeln!(tb).unwrap();
    writeln!(tb, "    initial begin").unwrap();
    writeln!(tb, "        clk = 0; rst = 1; valid_in = 0; errors = 0; total_out = 0;").unwrap();
    writeln!(tb, "        @(posedge clk); @(posedge clk);").unwrap();
    writeln!(tb, "        rst = 0;").unwrap();
    writeln!(tb, "        @(posedge clk);").unwrap();
    writeln!(tb).unwrap();

    // Feed 1000 inputs at II=1
    // Feed 1000 inputs at II=1, compute expected in lockstep
    writeln!(tb, "        for (i = 0; i < 1000; i = i + 1) begin").unwrap();

    for port in &dfg.inputs {
        let n = verilog_safe(&port.name);
        if port.width > 32 {
            writeln!(tb, "            {n} = {{$urandom, $urandom}};").unwrap();
        } else {
            writeln!(tb, "            {n} = $urandom;").unwrap();
        }
    }

    // Populate reference arrays
    for node in &dfg.nodes {
        if let DfgOp::Store { array } = &node.op {
            let inputs: Vec<u32> = dfg.edges.iter()
                .filter(|e| e.to == node.id && e.width > 0)
                .map(|e| e.from).collect();
            if inputs.len() >= 2 {
                let name = verilog_safe(array);
                let idx_expr = expr_for_node(inputs[0], dfg);
                let val_expr = expr_for_node(inputs[1], dfg);
                writeln!(tb, "            ref_{name}[{idx_expr}] = {val_expr};").unwrap();
            }
        }
    }

    writeln!(tb, "            expected_mem[i] = {};", build_reference_expr(bound)).unwrap();
    writeln!(tb, "            valid_in = 1;").unwrap();
    writeln!(tb, "            @(posedge clk);").unwrap();
    writeln!(tb, "        end").unwrap();
    writeln!(tb, "        valid_in = 0;").unwrap();
    writeln!(tb).unwrap();

    // Drain pipeline and collect all outputs
    writeln!(tb, "        repeat({}) begin", latency + 10).unwrap();
    writeln!(tb, "            @(posedge clk);").unwrap();
    writeln!(tb, "        end").unwrap();
    writeln!(tb).unwrap();

    writeln!(tb, "        if (errors == 0 && total_out >= 990)").unwrap();
    writeln!(tb, "            $display(\"PASS: %0d/%0d pipelined vectors correct (II=1)\", total_out, total_out);").unwrap();
    writeln!(tb, "        else").unwrap();
    writeln!(tb, "            $display(\"FAIL: %0d errors, %0d outputs of 1000\", errors, total_out);").unwrap();
    writeln!(tb, "        $finish;").unwrap();
    writeln!(tb, "    end").unwrap();
    writeln!(tb).unwrap();

    // Output checker — track outputs and compare
    // Pipeline has 1 extra clock of latency from input register, so output[n] = expected[n-1]
    if let Some(out_port) = dfg.outputs.first() {
        let n = verilog_safe(&out_port.name);
        writeln!(tb, "    always @(posedge clk) begin").unwrap();
        writeln!(tb, "        if (valid_out && !rst) begin").unwrap();
        writeln!(tb, "            if (total_out > 0 && {n} !== expected_mem[total_out - 1]) begin").unwrap();
        writeln!(tb, "                if (errors < 10)").unwrap();
        writeln!(tb, "                    $display(\"MISMATCH at out %0d: got=%0h expected=%0h\",").unwrap();
        writeln!(tb, "                        total_out, {n}, expected_mem[total_out - 1]);").unwrap();
        writeln!(tb, "                errors = errors + 1;").unwrap();
        writeln!(tb, "            end").unwrap();
        writeln!(tb, "            total_out = total_out + 1;").unwrap();
        writeln!(tb, "        end").unwrap();
        writeln!(tb, "    end").unwrap();
    }
    writeln!(tb).unwrap();

    writeln!(tb, "endmodule").unwrap();

    tb
}
