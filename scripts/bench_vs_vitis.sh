#!/bin/bash
# c2v vs Vitis HLS benchmark sweep
# Compiles each benchmark through c2v with verification,
# reports latency/area/compile-time for comparison.

set -e

C2V="./target/release/c2v"
RESULTS="/tmp/c2v_bench_results.txt"

if [ ! -f "$C2V" ]; then
    echo "Building c2v in release mode..."
    cargo build --release
fi

echo "c2v Benchmark Suite — $(date)" > "$RESULTS"
echo "=============================================" >> "$RESULTS"
printf "%-25s %6s %6s %6s %8s %s\n" \
    "Benchmark" "Cycles" "LUTs" "FFs" "Time(ms)" "Status" >> "$RESULTS"
echo "---------------------------------------------" >> "$RESULTS"

BENCHMARKS=(
    "examples/fd_add.c:fd_add_compute"
    "examples/fd_hash_round.c:fd_hash_round"
    "examples/fd_pipe_mac.c:fd_pipe_mac"
    "examples/fd_fir8.c:fd_fir8"
    "examples/fd_sha256_compress.c:fd_sha256_compress"
    "examples/fd_aes_sbox.c:fd_aes_sbox_lookup"
    "examples/fd_chacha_qr.c:fd_chacha_quarter_round"
    "examples/fd_blake3_round.c:fd_blake3_round"
    "examples/fd_crc8.c:fd_crc8_byte"
    "examples/fd_prefix_sum.c:fd_prefix_sum_8"
    "examples/fd_manhattan.c:fd_manhattan"
    "examples/fd_gcd.c:fd_gcd"
    "examples/fd_collatz.c:fd_collatz_steps"
    "examples/fd_fibonacci.c:fd_fibonacci"
    "examples/fd_bitreverse.c:fd_bitreverse32"
    "examples/fd_sort4.c:fd_sort4_packed"
    "examples/fd_cross_product.c:fd_cross_z"
    "examples/fd_ctz.c:fd_ctz32"
    "examples/fd_base_convert.c:fd_to_bcd"
    "examples/fd_interval_overlap.c:fd_interval_overlap"
    "examples/fd_hamming_dist.c:fd_hamming_dist"
    "examples/fd_majority.c:fd_majority5"
    "examples/fd_matrix_det2x2.c:fd_det2x2"
    "examples/fd_dot4.c:fd_dot4"
    "examples/fd_matmul2x2.c:fd_matmul2x2_00"
    "examples/fd_median3.c:fd_median3"
    "examples/fd_cordic.c:fd_cordic_step"
    "examples/fd_popcount64.c:fd_popcount64"
    "examples/fd_endian_swap.c:fd_bswap32"
    "examples/fd_barrel_shift.c:fd_barrel_shift"
)

PASS=0
FAIL=0
TOTAL=${#BENCHMARKS[@]}

for entry in "${BENCHMARKS[@]}"; do
    FILE="${entry%%:*}"
    FUNC="${entry##*:}"
    OUT="/tmp/c2v_bench_${FUNC}.v"

    START=$(date +%s%N)
    STDERR=$($C2V "$FILE" -f "$FUNC" --verify -o "$OUT" 2>&1) || true
    END=$(date +%s%N)
    ELAPSED=$(( (END - START) / 1000000 ))

    CYCLES=$(echo "$STDERR" | grep "\[schedule\]" | sed 's/.*→ \([0-9]*\) cycles.*/\1/' || echo "?")
    LUTS=$(echo "$STDERR" | grep "Area est" | sed 's/.*~\([0-9]*\) LUTs.*/\1/' || echo "?")
    FFS=$(echo "$STDERR" | grep "Area est" | sed 's/.*, \([0-9]*\) FFs.*/\1/' || echo "?")

    if echo "$STDERR" | grep -q "1000/1000 correct"; then
        STATUS="✓ PASS"
        PASS=$((PASS + 1))
    else
        STATUS="✗ FAIL"
        FAIL=$((FAIL + 1))
    fi

    printf "%-25s %6s %6s %6s %8s %s\n" \
        "$FUNC" "$CYCLES" "$LUTS" "$FFS" "$ELAPSED" "$STATUS" >> "$RESULTS"

    rm -f "$OUT" "${OUT%.v}.tb.v"
done

echo "---------------------------------------------" >> "$RESULTS"
echo "Total: $TOTAL  Pass: $PASS  Fail: $FAIL" >> "$RESULTS"
echo "" >> "$RESULTS"

cat "$RESULTS"
