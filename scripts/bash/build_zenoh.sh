#!/bin/bash

if test "$#" -ne 1; then
    echo "Please provide Working Directory"
    exit 1
fi

BWD=$1

mkdir $BWD
pushd
cd $BWD
rustup update
git clone https://github.com/eclipse-zenoh/zenoh.git
cd zenoh
git checkout rust-master
cargo build --release --examples
popd &> /dev/null
