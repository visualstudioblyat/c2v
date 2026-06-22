use std::fmt::Write;

use crate::ir::BoundDfg;

pub fn generate_waveform_testbench(bound: &BoundDfg) -> String {
    let dfg = &bound.scheduled.dfg;
    let mut tb = String::with_capacity(4096);
    let mod_name = super::verilog_safe_name(&dfg.name);

    writeln!(tb, "// Waveform testbench for {} — generates VCD for GTKWave", dfg.name).unwrap();
    writeln!(tb, "`timescale 1ns/1ps").unwrap();
    writeln!(tb).unwrap();
    writeln!(tb, "module tb_wave_{mod_name};").unwrap();
    writeln!(tb).unwrap();
    writeln!(tb, "    reg clk, rst, start;").unwrap();
    writeln!(tb, "    wire done;").unwrap();

    for port in &dfg.inputs {
        let n = super::verilog_safe_name(&port.name);
        writeln!(tb, "    reg  [{w}:0] {n};", w = port.width - 1).unwrap();
    }
    for port in &dfg.outputs {
        let n = super::verilog_safe_name(&port.name);
        writeln!(tb, "    wire [{w}:0] {n};", w = port.width - 1).unwrap();
    }
    writeln!(tb).unwrap();

    writeln!(tb, "    {mod_name} dut(").unwrap();
    writeln!(tb, "        .clk(clk), .rst(rst), .start(start), .done(done),").unwrap();
    let all_ports: Vec<_> = dfg.inputs.iter().chain(dfg.outputs.iter()).collect();
    for (i, port) in all_ports.iter().enumerate() {
        let n = super::verilog_safe_name(&port.name);
        let comma = if i == all_ports.len() - 1 { "" } else { "," };
        writeln!(tb, "        .{n}({n}){comma}").unwrap();
    }
    writeln!(tb, "    );").unwrap();
    writeln!(tb).unwrap();

    writeln!(tb, "    always #5 clk = ~clk;").unwrap();
    writeln!(tb).unwrap();

    writeln!(tb, "    initial begin").unwrap();
    writeln!(tb, "        $dumpfile(\"{mod_name}.vcd\");").unwrap();
    writeln!(tb, "        $dumpvars(0, tb_wave_{mod_name});").unwrap();
    writeln!(tb).unwrap();
    writeln!(tb, "        clk = 0; rst = 1; start = 0;").unwrap();
    writeln!(tb, "        #20 rst = 0;").unwrap();
    writeln!(tb, "        #10;").unwrap();
    writeln!(tb).unwrap();

    writeln!(tb, "        // Test vector 1").unwrap();
    for (i, port) in dfg.inputs.iter().enumerate() {
        let n = super::verilog_safe_name(&port.name);
        writeln!(tb, "        {n} = {};", i + 1).unwrap();
    }
    let wait = bound.scheduled.total_cycles * 2 + 5;
    writeln!(tb, "        start = 1; #10 start = 0;").unwrap();
    writeln!(tb, "        repeat({wait}) @(posedge clk);").unwrap();
    writeln!(tb).unwrap();

    writeln!(tb, "        // Test vector 2 (random)").unwrap();
    for port in &dfg.inputs {
        let n = super::verilog_safe_name(&port.name);
        if port.width > 32 {
            writeln!(tb, "        {n} = {{$urandom, $urandom}};").unwrap();
        } else {
            writeln!(tb, "        {n} = $urandom;").unwrap();
        }
    }
    writeln!(tb, "        start = 1; #10 start = 0;").unwrap();
    writeln!(tb, "        repeat({wait}) @(posedge clk);").unwrap();
    writeln!(tb).unwrap();

    writeln!(tb, "        $finish;").unwrap();
    writeln!(tb, "    end").unwrap();
    writeln!(tb).unwrap();
    writeln!(tb, "endmodule").unwrap();

    tb
}
