#!/usr/bin/env bash

set -e

mkdir -p ../fixtures/streaming/v10
(
    cd streaming/v10
    mkdir -p build
    rm -rf build/*
    cd build
    cmake -DCMAKE_BUILD_TYPE=Debug ..
    make run
)

exit 0
