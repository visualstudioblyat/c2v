#!/bin/bash
# sweep.sh — explore design space for a function
# Usage: ./scripts/sweep.sh examples/fd_fir8_parallel.c fd_fir8_parallel
#
# Sweeps multiplier count from 1 to 8, collects cycles + area for each.
# Output: JSON array for ML consumption.

set -euo pipefail

SRC="${1:?usage: sweep.sh <source.c> <function>}"
FUNC="${2:?usage: sweep.sh <source.c> <function>}"
C2V="./target/release/c2v"

if [ ! -f "$C2V" ]; then
    echo "Building release..." >&2
    cargo build --release 2>/dev/null
fi

echo "["

first=true
for mul in 1 2 4 8; do
    for add in 2 4 8; do
        for bal in "" "--balance"; do
            OUT="/tmp/c2v_sweep_${FUNC}_m${mul}_a${add}${bal:+_bal}"

            result=$($C2V "$SRC" -f "$FUNC" \
                --max-mul "$mul" --max-add "$add" $bal \
                --trace -o "${OUT}.v" 2>&1)

            cycles=$(echo "$result" | grep -oP '\d+(?= cycles)' | head -1)
            luts=$(echo "$result" | grep "Area" | grep -oP '~\K\d+' | head -1)
            dsps=$(echo "$result" | grep "Area" | grep -oP '\d+(?= DSP)' | head -1)

            if [ "$first" = true ]; then first=false; else echo ","; fi
            printf '  {"mul":%d,"add":%d,"balance":%s,"cycles":%s,"luts":%s,"dsps":%s}' \
                "$mul" "$add" \
                "$([ -n "$bal" ] && echo true || echo false)" \
                "${cycles:-0}" "${luts:-0}" "${dsps:-0}"

            rm -f "${OUT}.v" "${OUT}.trace.json"
        done
    done
done

echo ""
echo "]"
