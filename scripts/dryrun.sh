#!/bin/bash

# Copyright (c) Mysten Labs, Inc.
# SPDX-License-Identifier: Apache-2.0

export RUST_LOG=warn,mysticeti_core::consensus=trace,mysticeti_core::net_sync=DEBUG,mysticeti_core::core=DEBUG,RUST_BACKTRACE=1

for i in {0..9}; do
    echo "Starting run $i..."

    mkdir -p "runs"
    rm -rf "runs/r$i"
    mkdir -p "runs/r$i"

    for s in v0 v1 v2 v3; do
        tmux kill-session -t $s 2>/dev/null || true
    done

    tmux new -d -s "v0" "cargo run --bin mysticeti -- dry-run --committee-size 4 --authority 0 > runs/r$i/v0.log.ansi"
    tmux new -d -s "v1" "cargo run --bin mysticeti -- dry-run --committee-size 4 --authority 1 > runs/r$i/v1.log.ansi"
    tmux new -d -s "v2" "cargo run --bin mysticeti -- dry-run --committee-size 4 --authority 2 > runs/r$i/v2.log.ansi"
    tmux new -d -s "v3" "cargo run --bin mysticeti -- dry-run --committee-size 4 --authority 3 > runs/r$i/v3.log.ansi"

    sleep 120
    for s in v0 v1 v2 v3; do
        tmux kill-session -t $s 2>/dev/null || true
    done

    echo "Run $i completed."
done
