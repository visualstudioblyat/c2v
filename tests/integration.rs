use std::process::Command;

fn c2v(args: &[&str]) -> (String, String, bool) {
    let output = Command::new(env!("CARGO_BIN_EXE_c2v"))
        .args(args)
        .output()
        .expect("failed to run c2v");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (stdout, stderr, output.status.success())
}

fn unique_output(func: &str, suffix: &str) -> String {
    let id = std::process::id();
    let tid: u64 = unsafe { std::mem::transmute(std::thread::current().id()) };
    format!("/tmp/c2v_test_{func}_{id}_{tid}{suffix}")
}

fn compile_and_verify(file: &str, func: &str) -> String {
    let out_path = unique_output(func, ".v");
    let (_, stderr, success) = c2v(&[file, "-f", func, "--verify", "-o", &out_path]);
    let _ = std::fs::remove_file(&out_path);
    let _ = std::fs::remove_file(format!("{}.tb.v", out_path.trim_end_matches(".v")));
    let _ = std::fs::remove_file(out_path.replace(".v", ".verify.v"));
    assert!(success || stderr.contains("REJECTED"), "c2v failed:\n{stderr}");
    stderr
}

fn compile_with_balance(file: &str, func: &str) -> String {
    let out_path = unique_output(func, "_bal.v");
    let (_, stderr, success) = c2v(&[file, "-f", func, "--verify", "--balance", "-o", &out_path]);
    let _ = std::fs::remove_file(&out_path);
    let _ = std::fs::remove_file(format!("{}.tb.v", out_path.trim_end_matches(".v")));
    assert!(success, "c2v failed:\n{stderr}");
    stderr
}

fn assert_verified(stderr: &str) {
    assert!(stderr.contains("1000/1000 correct"), "verification failed:\n{stderr}");
}

fn assert_rejected(stderr: &str) {
    assert!(stderr.contains("REJECTED"), "should have been rejected:\n{stderr}");
}

fn extract_cycles(stderr: &str) -> u32 {
    stderr.lines()
        .find(|l| l.contains("[schedule]"))
        .and_then(|l| {
            l.split('→').nth(1)
                .and_then(|s| s.trim().split_whitespace().next())
                .and_then(|s| s.parse().ok())
        })
        .expect("couldn't extract cycle count")
}

// ─── Basic arithmetic ───

#[test]
fn test_add() {
    let out = compile_and_verify("examples/fd_add.c", "fd_add_compute");
    assert_verified(&out);
}

#[test]
fn test_hash_round() {
    let out = compile_and_verify("examples/fd_hash_round.c", "fd_hash_round");
    assert_verified(&out);
}

#[test]
fn test_pipe_mac() {
    let out = compile_and_verify("examples/fd_pipe_mac.c", "fd_pipe_mac");
    assert_verified(&out);
}

// ─── Loops ───

#[test]
fn test_fir8_serial() {
    let out = compile_and_verify("examples/fd_fir8.c", "fd_fir8_compute");
    assert_verified(&out);
}

#[test]
fn test_fir8_parallel() {
    let out = compile_and_verify("examples/fd_fir8_parallel.c", "fd_fir8_parallel");
    assert_verified(&out);
}

#[test]
fn test_fir8_loop() {
    let out = compile_and_verify("examples/fd_fir8_loop.c", "fd_fir8_loop");
    assert_verified(&out);
}

#[test]
fn test_sum16() {
    let out = compile_and_verify("examples/fd_sum16.c", "fd_sum16");
    assert_verified(&out);
}

// ─── Arrays / Memory ───

#[test]
fn test_lut() {
    let out = compile_and_verify("examples/fd_lut.c", "fd_lut_compute");
    assert_verified(&out);
}

#[test]
fn test_dot4() {
    let out = compile_and_verify("examples/fd_dot4.c", "fd_dot4_compute");
    assert_verified(&out);
}

#[test]
fn test_matmul2x2() {
    let out = compile_and_verify("examples/fd_matmul2x2.c", "fd_matmul2x2");
    assert_verified(&out);
}

#[test]
fn test_aes_sbox() {
    let out = compile_and_verify("examples/fd_aes_sbox.c", "fd_aes_sbox_round");
    assert_verified(&out);
}

// ─── Structs ───

#[test]
fn test_point() {
    let out = compile_and_verify("examples/fd_point.c", "fd_point_dot");
    assert_verified(&out);
}

#[test]
fn test_packet() {
    let out = compile_and_verify("examples/fd_packet.c", "fd_packet_checksum");
    assert_verified(&out);
}

// ─── Control flow ───

#[test]
fn test_max() {
    let out = compile_and_verify("examples/fd_max.c", "fd_max");
    assert_verified(&out);
}

#[test]
fn test_clamp() {
    let out = compile_and_verify("examples/fd_clamp.c", "fd_clamp");
    assert_verified(&out);
}

#[test]
fn test_abs_diff() {
    let out = compile_and_verify("examples/fd_abs_diff.c", "fd_abs_diff");
    assert_verified(&out);
}

// ─── Inlining ───

#[test]
fn test_inline() {
    let out = compile_and_verify("examples/fd_inline.c", "fd_hash2");
    assert_verified(&out);
}

// ─── Coding standard rejection ───

#[test]
fn test_reject_malloc() {
    let out = compile_and_verify("examples/fd_bad_malloc.c", "bad_function");
    assert_rejected(&out);
}

// ─── Tree balancing (Q1 Shannon optimization) ───

#[test]
fn test_balance_sum16() {
    let unbalanced = compile_and_verify("examples/fd_sum16.c", "fd_sum16");
    let balanced = compile_with_balance("examples/fd_sum16.c", "fd_sum16");
    assert_verified(&unbalanced);
    assert_verified(&balanced);
    let cyc_before = extract_cycles(&unbalanced);
    let cyc_after = extract_cycles(&balanced);
    assert!(cyc_after < cyc_before, "balance should reduce cycles: {cyc_before} -> {cyc_after}");
}

#[test]
fn test_balance_fir() {
    let unbalanced = compile_and_verify("examples/fd_fir8.c", "fd_fir8_compute");
    let balanced = compile_with_balance("examples/fd_fir8.c", "fd_fir8_compute");
    assert_verified(&balanced);
    let cyc_before = extract_cycles(&unbalanced);
    let cyc_after = extract_cycles(&balanced);
    assert!(cyc_after < cyc_before, "balance should reduce cycles: {cyc_before} -> {cyc_after}");
}

#[test]
fn test_balance_sha256() {
    let balanced = compile_with_balance("examples/fd_hash_round.c", "fd_hash_round");
    assert_verified(&balanced);
    let cyc = extract_cycles(&balanced);
    assert!(cyc <= 8, "balanced SHA-256 should be <= 8 cycles, got {cyc}");
}

// ─── Resource constraints ───

#[test]
fn test_resource_constrained() {
    let out_path = unique_output("fir8_rc", ".v");
    let (_, stderr, success) = c2v(&[
        "examples/fd_fir8_parallel.c", "-f", "fd_fir8_parallel",
        "--verify", "--max-mul", "2", "-o", &out_path
    ]);
    let _ = std::fs::remove_file(&out_path);
    let _ = std::fs::remove_file(format!("{}.tb.v", out_path.trim_end_matches(".v")));
    assert!(success, "c2v failed:\n{stderr}");
    assert_verified(&stderr);
    let cyc = extract_cycles(&stderr);
    assert!(cyc > 9, "with 2 multipliers should be > 9 cycles, got {cyc}");
}

// ─── Complex control flow ───

#[test]
fn test_classify() {
    let out = compile_and_verify("examples/fd_classify.c", "fd_classify");
    assert_verified(&out);
}

// ─── Nested structs ───

#[test]
fn test_nested_struct() {
    let out = compile_and_verify("examples/fd_nested_struct.c", "fd_body_energy");
    assert_verified(&out);
}

#[test]
fn test_bitcount() {
    let out = compile_and_verify("examples/fd_bitcount.c", "fd_bitcount");
    assert_verified(&out);
}

#[test]
fn test_minmax3() {
    let out = compile_and_verify("examples/fd_minmax3.c", "fd_minmax3");
    assert_verified(&out);
}

#[test]
fn test_crc8() {
    let out = compile_and_verify("examples/fd_crc8.c", "fd_crc8");
    assert_verified(&out);
}

#[test]
fn test_multi_inline() {
    let out = compile_and_verify("examples/fd_multi_inline.c", "fd_double_round");
    assert_verified(&out);
}

#[test]
fn test_bitcount_balanced() {
    let out = compile_with_balance("examples/fd_bitcount.c", "fd_bitcount");
    assert_verified(&out);
}

#[test]
fn test_median3() {
    let out = compile_and_verify("examples/fd_median3.c", "fd_median3");
    assert_verified(&out);
}

#[test]
fn test_alu() {
    let out = compile_and_verify("examples/fd_alu.c", "fd_alu");
    assert_verified(&out);
}

#[test]
fn test_priority_enc() {
    let out = compile_and_verify("examples/fd_priority_enc.c", "fd_priority_enc");
    assert_verified(&out);
}

#[test]
fn test_scramble() {
    let out = compile_and_verify("examples/fd_scramble.c", "fd_scramble");
    assert_verified(&out);
}

#[test]
fn test_cordic() {
    let out = compile_and_verify("examples/fd_cordic.c", "fd_cordic");
    assert_verified(&out);
}

#[test]
fn test_gray() {
    let out = compile_and_verify("examples/fd_gray.c", "fd_gray_roundtrip");
    assert_verified(&out);
}

#[test]
fn test_fifo_ctrl() {
    let out = compile_and_verify("examples/fd_fifo_ctrl.c", "fd_fifo_push_pop");
    assert_verified(&out);
}

#[test]
fn test_lfsr() {
    let out = compile_and_verify("examples/fd_lfsr.c", "fd_lfsr_advance");
    assert_verified(&out);
}

#[test]
fn test_matrix_vec() {
    let out = compile_and_verify("examples/fd_matrix_vec.c", "fd_mat4_vec4");
    assert_verified(&out);
}

#[test]
fn test_iir_biquad() {
    let out = compile_and_verify("examples/fd_iir_biquad.c", "fd_iir_biquad");
    assert_verified(&out);
}

#[test]
fn test_pid() {
    let out = compile_and_verify("examples/fd_pid.c", "fd_pid_step");
    assert_verified(&out);
}

#[test]
fn test_multifile() {
    let out_path = unique_output("fd_blend", ".v");
    let (_, stderr, success) = c2v(&[
        "examples/fd_helpers.c", "examples/fd_multifile.c",
        "-f", "fd_blend", "--verify", "-o", &out_path
    ]);
    let _ = std::fs::remove_file(&out_path);
    let _ = std::fs::remove_file(format!("{}.tb.v", out_path.trim_end_matches(".v")));
    assert!(success, "multi-file failed:\n{stderr}");
    assert_verified(&stderr);
}

#[test]
fn test_convolution() {
    let out = compile_and_verify("examples/fd_convolution.c", "fd_conv3x3");
    assert_verified(&out);
}

#[test]
fn test_ethernet_crc() {
    let out = compile_and_verify("examples/fd_ethernet_crc.c", "fd_eth_crc32_byte");
    assert_verified(&out);
}

#[test]
fn test_spi_master() {
    let out = compile_and_verify("examples/fd_spi_master.c", "fd_spi_shift_byte");
    assert_verified(&out);
}

#[test]
fn test_sha256_compress() {
    let out = compile_with_balance("examples/fd_sha256_compress.c", "fd_sha256_compress4");
    assert_verified(&out);
}

#[test]
fn test_uart_rx() {
    let out = compile_and_verify("examples/fd_uart_rx.c", "fd_uart_rx_tick");
    assert_verified(&out);
}

#[test]
fn test_huffman() {
    let out = compile_and_verify("examples/fd_huffman.c", "fd_huffman_decode");
    assert_verified(&out);
}

#[test]
fn test_relu() {
    let out = compile_with_balance("examples/fd_relu.c", "fd_relu_linear4");
    assert_verified(&out);
}

#[test]
fn test_pwm() {
    let out = compile_and_verify("examples/fd_pwm.c", "fd_pwm_tick");
    assert_verified(&out);
}

#[test]
fn test_convolution_balanced() {
    let out = compile_with_balance("examples/fd_convolution.c", "fd_conv3x3");
    assert_verified(&out);
}

#[test]
fn test_i2c_master() {
    let out = compile_and_verify("examples/fd_i2c_master.c", "fd_i2c_shift_byte");
    assert_verified(&out);
}

#[test]
fn test_opcode_switch() {
    let out = compile_and_verify("examples/fd_opcode.c", "fd_opcode_exec");
    assert_verified(&out);
}

#[test]
fn test_sha1_round() {
    let out = compile_and_verify("examples/fd_sha1_round.c", "fd_sha1_round");
    assert_verified(&out);
}

#[test]
fn test_aes_full_round() {
    let out = compile_and_verify("examples/fd_aes_round.c", "fd_aes_round");
    assert_verified(&out);
}

#[test]
fn test_sha1_multi() {
    let out = compile_and_verify("examples/fd_sha1_multi.c", "fd_sha1_multi");
    assert_verified(&out);
}

#[test]
fn test_arbiter() {
    let out = compile_and_verify("examples/fd_arbiter.c", "fd_arbiter_rr4");
    assert_verified(&out);
}

#[test]
fn test_blowfish_round() {
    let out = compile_and_verify("examples/fd_blowfish_round.c", "fd_blowfish_round");
    assert_verified(&out);
}

#[test]
fn test_adpcm() {
    let out = compile_and_verify("examples/fd_adpcm_step.c", "fd_adpcm_encode");
    assert_verified(&out);
}
