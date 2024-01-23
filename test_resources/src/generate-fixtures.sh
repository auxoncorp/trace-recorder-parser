#!/usr/bin/env bash

set -e

for trace_fmt_version_dir in streaming/*; do
    mkdir -p ../fixtures/${trace_fmt_version_dir}
    (
        cd ${trace_fmt_version_dir}
        mkdir -p build
        rm -rf build/*
        cd build
        cmake -DCMAKE_BUILD_TYPE=Debug ..
        make run
    )
done

exit 0
