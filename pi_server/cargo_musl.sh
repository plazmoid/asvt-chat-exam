#!/bin/bash
docker run \
    -v $PWD:/volume \
    -v cargo-cache:/root/.cargo/registry \
    -w /volume \
    -it \
    clux/muslrust \
    cargo $@
