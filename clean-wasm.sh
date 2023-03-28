#!/bin/sh

# cleans all wasm targets

cargo install multiversx-sc-meta

sc-meta all clean --path ./contracts


# not wasm, but worth cleaning from time to time

cargo clean
