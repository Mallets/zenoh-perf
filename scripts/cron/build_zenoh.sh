#!/bin/bash

DN="$(dirname $0)"
rustup update
pushd
mkdir $DN/temp
cd $DN/temp
git clone https://github.com/eclipse-zenoh/zenoh.git
cd zenoh
git checkout rust-master
cargo build --release --examples
popd &> /dev/null
