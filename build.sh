#!/bin/bash

set -e

version=$(yq -oy '.workspace.package.version' Cargo.toml);
echo "building pong client and server version '$version'";
docker build -t pong-server:$version -f server.prod.Dockerfile .;
cargo build --release --bin client --target x86_64-unknown-linux-gnu;
cargo build --release --bin client --target x86_64-pc-windows-gnu;
