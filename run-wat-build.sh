#!/bin/bash

export PATH=/home/elrond/Github/wabt/build:$PATH

./wat-build.sh locked-asset/factory
./wat-build.sh locked-asset/distribution
./wat-build.sh locked-asset/proxy_dex

./wat-build.sh dex/pair
./wat-build.sh dex/router
./wat-build.sh dex/farm
./wat-build.sh dex/farm-with-lock
./wat-build.sh dex/farm-staking
./wat-build.sh dex/farm-staking-proxy

./wat-build.sh price-discovery
