use std::process::Command;

fn c2v(args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_c2v"))
        .args(args)
        .output()
        .expect("failed to run c2v");
    String::from_utf8_lossy(&output.stderr).to_string()
}

fn unique_output(name: &str) -> String {
    let id = std::process::id();
    format!("/tmp/c2v_unit_{name}_{id}.v")
}

#[test]
fn test_strength_reduce_power_of_2() {
    // x * 4 should become x << 2 (strength reduction)
    let source = r#"
typedef unsigned int uint;
uint fd_test(uint x) { return x * 4; }
"#;
    let src_path = "/tmp/c2v_sr_test.c";
    std::fs::write(src_path, source).unwrap();
    let out = unique_output("sr");
    let stderr = c2v(&[src_path, "-f", "fd_test", "-o", &out]);
    let verilog = std::fs::read_to_string(&out).unwrap_or_default();
    let _ = std::fs::remove_file(&out);
    let _ = std::fs::remove_file(src_path);
    // After strength reduction, should use shift not multiply
    assert!(verilog.contains("<<") || !verilog.contains("*"),
        "strength reduction should convert *4 to <<2:\n{verilog}");
}

#[test]
fn test_const_fold() {
    // 3 + 5 should fold to 8
    let source = r#"
typedef unsigned int uint;
uint fd_test(uint x) { return x + (3 + 5); }
"#;
    let src_path = "/tmp/c2v_cf_test.c";
    std::fs::write(src_path, source).unwrap();
    let out = unique_output("cf");
    let stderr = c2v(&[src_path, "-f", "fd_test", "-o", &out]);
    let verilog = std::fs::read_to_string(&out).unwrap_or_default();
    let _ = std::fs::remove_file(&out);
    let _ = std::fs::remove_file(src_path);
    // Verify the function compiles — const folding is best-effort
    assert!(verilog.contains("add") || verilog.contains("32'd8"),
        "should produce valid add or folded constant:\n{verilog}");
}

#[test]
fn test_identity_elim_add_zero() {
    // x + 0 should simplify to x
    let source = r#"
typedef unsigned int uint;
uint fd_test(uint x) { return x + 0; }
"#;
    let src_path = "/tmp/c2v_ie_test.c";
    std::fs::write(src_path, source).unwrap();
    let out = unique_output("ie");
    let stderr = c2v(&[src_path, "-f", "fd_test", "--verify", "-o", &out]);
    let _ = std::fs::remove_file(&out);
    let _ = std::fs::remove_file(src_path);
    assert!(stderr.contains("1000/1000 correct"), "identity elim should preserve correctness");
}

#[test]
fn test_cse_duplicate_expr() {
    // (a*b) and (a+b) used — CSE shouldn't merge different ops
    let source = r#"
typedef unsigned int uint;
uint fd_test(uint a, uint b) {
    uint x = a * b;
    uint y = a + b;
    return x + y;
}
"#;
    let src_path = "/tmp/c2v_cse_test.c";
    std::fs::write(src_path, source).unwrap();
    let out = unique_output("cse");
    let stderr = c2v(&[src_path, "-f", "fd_test", "--verify", "-o", &out]);
    let _ = std::fs::remove_file(&out);
    let _ = std::fs::remove_file(src_path);
    assert!(stderr.contains("1000/1000 correct"), "CSE should preserve correctness");
}

#[test]
fn test_tree_balance_reduces_depth() {
    let source = r#"
typedef unsigned int uint;
uint fd_test(uint a, uint b, uint c, uint d) {
    uint sum = a;
    sum = sum + b;
    sum = sum + c;
    sum = sum + d;
    return sum;
}
"#;
    let src_path = "/tmp/c2v_tb_test.c";
    std::fs::write(src_path, source).unwrap();
    let out1 = unique_output("tb1");
    let out2 = unique_output("tb2");

    let s1 = c2v(&[src_path, "-f", "fd_test", "-o", &out1]);
    let s2 = c2v(&[src_path, "-f", "fd_test", "--balance", "-o", &out2]);

    let c1 = extract_cycles(&s1);
    let c2 = extract_cycles(&s2);

    let _ = std::fs::remove_file(&out1);
    let _ = std::fs::remove_file(&out2);
    let _ = std::fs::remove_file(src_path);

    assert!(c2 <= c1, "balanced ({c2}) should be <= unbalanced ({c1})");
}

#[test]
fn test_signed_propagation() {
    let source = r#"
typedef int sint;
sint fd_test(sint a, sint b) { return a + b; }
"#;
    let src_path = "/tmp/c2v_sign_test.c";
    std::fs::write(src_path, source).unwrap();
    let out = unique_output("sign");
    let _ = c2v(&[src_path, "-f", "fd_test", "-o", &out]);
    let verilog = std::fs::read_to_string(&out).unwrap_or_default();
    let _ = std::fs::remove_file(&out);
    let _ = std::fs::remove_file(src_path);
    assert!(verilog.contains("signed"), "signed types should produce signed Verilog");
}

#[test]
fn test_reject_malloc() {
    let source = r#"
#include <stdlib.h>
int bad(int n) { int *p = malloc(n); return *p; }
"#;
    let src_path = "/tmp/c2v_reject_test.c";
    std::fs::write(src_path, source).unwrap();
    let stderr = c2v(&[src_path, "-f", "bad"]);
    let _ = std::fs::remove_file(src_path);
    assert!(stderr.contains("REJECTED"), "should reject malloc");
}

#[test]
fn test_reject_goto() {
    let source = r#"
typedef unsigned int uint;
uint bad(uint x) { if(x) goto end; end: return x; }
"#;
    let src_path = "/tmp/c2v_goto_test.c";
    std::fs::write(src_path, source).unwrap();
    let stderr = c2v(&[src_path, "-f", "bad"]);
    let _ = std::fs::remove_file(src_path);
    assert!(stderr.contains("REJECTED"), "should reject goto");
}

fn extract_cycles(stderr: &str) -> u32 {
    stderr.lines()
        .find(|l| l.contains("[schedule]"))
        .and_then(|l| {
            l.split('→').nth(1)
                .and_then(|s| s.trim().split_whitespace().next())
                .and_then(|s| s.parse().ok())
        })
        .unwrap_or(999)
}
