#!/bin/bash
set -e

ROOT=`pwd`

cd conditional-escrow
sh build.sh
cargo test -- --nocapture --exact
cd $ROOT

cd escrow
sh build.sh
cargo test -- --nocapture --exact
cd $ROOT

cd dao-factory
sh build.sh
cargo test -- --nocapture --exact
cd $ROOT

cd fungible-token
sh build.sh
cargo test -- --nocapture --exact
cd $ROOT

cd ft-factory
sh build.sh
cargo test -- --nocapture --exact
cd $ROOT

cd staking-factory
sh build.sh
cargo test -- --nocapture --exact
cd $ROOT

cargo test -- --nocapture --exact
RUSTFLAGS='-C link-arg=-s' cargo build --target wasm32-unknown-unknown --release
cp ./target/wasm32-unknown-unknown/release/escrow_factory.wasm src/
