#!/bin/sh

# bytecode sizes

stat --printf="dex/farm %s\n" dex/farm/output/farm.wasm
stat --printf="dex/pair %s\n" dex/pair/output/pair.wasm
stat --printf="dex/router %s\n" dex/router/output/router.wasm
stat --printf="locked-asset/distribution %s\n" locked-asset/distribution/output/distribution.wasm
stat --printf="locked-asset/proxy_dex %s\n" locked-asset/proxy_dex/output/proxy_dex.wasm
stat --printf="locked-asset/factory %s\n" locked-asset/factory/output/factory.wasm
