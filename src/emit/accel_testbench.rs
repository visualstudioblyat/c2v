use std::fmt::Write;
use crate::ir::BoundDfg;

pub fn emit_accel_testbench(bound: &BoundDfg, input_port: &str, output_port: &str, depth: u32) -> String {
    let mut tb = String::with_capacity(4096);
    let dfg = &bound.scheduled.dfg;
    let name = super::verilog_safe_name(&dfg.name);
    let abits = (32 - (depth - 1).leading_zeros()).max(1);

    writeln!(tb, "`timescale 1ns/1ps").unwrap();
    writeln!(tb, "module tb_{name}_stream;").unwrap();
    writeln!(tb, "    reg clk, rst, start;").unwrap();
    writeln!(tb, "    wire done, idle;").unwrap();
    writeln!(tb, "    wire [{a}:0] {input_port}_addr;", a = abits - 1).unwrap();
    writeln!(tb, "    reg [31:0] {input_port}_rdata;").unwrap();
    writeln!(tb, "    wire [{a}:0] {output_port}_addr;", a = abits - 1).unwrap();
    writeln!(tb, "    wire [31:0] {output_port}_wdata;").unwrap();
    writeln!(tb, "    wire {output_port}_we;").unwrap();
    writeln!(tb).unwrap();

    writeln!(tb, "    reg [31:0] input_mem [0:{d}];", d = depth - 1).unwrap();
    writeln!(tb, "    reg [31:0] output_mem [0:{d}];", d = depth - 1).unwrap();
    writeln!(tb).unwrap();

    writeln!(tb, "    {name}_stream #(.N({depth}), .AWIDTH({abits})) dut (").unwrap();
    writeln!(tb, "        .clk(clk), .rst(rst), .start(start),").unwrap();
    writeln!(tb, "        .done(done), .idle(idle),").unwrap();
    writeln!(tb, "        .{input_port}_addr({input_port}_addr),").unwrap();
    writeln!(tb, "        .{input_port}_rdata({input_port}_rdata),").unwrap();
    writeln!(tb, "        .{output_port}_addr({output_port}_addr),").unwrap();
    writeln!(tb, "        .{output_port}_wdata({output_port}_wdata),").unwrap();
    writeln!(tb, "        .{output_port}_we({output_port}_we)").unwrap();
    writeln!(tb, "    );").unwrap();
    writeln!(tb).unwrap();

    writeln!(tb, "    always #5 clk = ~clk;").unwrap();
    writeln!(tb).unwrap();

    writeln!(tb, "    always @(posedge clk)").unwrap();
    writeln!(tb, "        {input_port}_rdata <= input_mem[{input_port}_addr];").unwrap();
    writeln!(tb).unwrap();

    writeln!(tb, "    always @(posedge clk)").unwrap();
    writeln!(tb, "        if ({output_port}_we) output_mem[{output_port}_addr] <= {output_port}_wdata;").unwrap();
    writeln!(tb).unwrap();

    writeln!(tb, "    integer i, errors;").unwrap();
    writeln!(tb, "    reg [31:0] expected;").unwrap();
    writeln!(tb).unwrap();

    writeln!(tb, "    initial begin").unwrap();
    writeln!(tb, "        clk = 0; rst = 1; start = 0; errors = 0;").unwrap();
    writeln!(tb, "        for (i = 0; i < {depth}; i = i + 1)").unwrap();
    writeln!(tb, "            input_mem[i] = i + 1;").unwrap();
    writeln!(tb, "        @(posedge clk); @(posedge clk);").unwrap();
    writeln!(tb, "        rst = 0;").unwrap();
    writeln!(tb, "        @(posedge clk);").unwrap();
    writeln!(tb, "        start = 1;").unwrap();
    writeln!(tb, "        @(posedge clk);").unwrap();
    writeln!(tb, "        start = 0;").unwrap();
    writeln!(tb).unwrap();

    writeln!(tb, "        wait(done);").unwrap();
    writeln!(tb, "        @(posedge clk); @(posedge clk);").unwrap();
    writeln!(tb).unwrap();

    writeln!(tb, "        for (i = 0; i < {depth}; i = i + 1) begin").unwrap();
    writeln!(tb, "            if (output_mem[i] === 32'bx) begin").unwrap();
    writeln!(tb, "                $display(\"MISSING at %0d\", i);").unwrap();
    writeln!(tb, "                errors = errors + 1;").unwrap();
    writeln!(tb, "            end").unwrap();
    writeln!(tb, "        end").unwrap();
    writeln!(tb).unwrap();

    writeln!(tb, "        if (errors == 0)").unwrap();
    writeln!(tb, "            $display(\"ACCEL PASS: {depth}/{depth} elements processed\");").unwrap();
    writeln!(tb, "        else").unwrap();
    writeln!(tb, "            $display(\"ACCEL FAIL: %0d errors\", errors);").unwrap();
    writeln!(tb, "        $finish;").unwrap();
    writeln!(tb, "    end").unwrap();
    writeln!(tb, "endmodule").unwrap();

    tb
}
