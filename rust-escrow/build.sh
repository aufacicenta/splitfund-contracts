#!/bin/bash
set -e

ROOT=`pwd`

cd conditional-escrow
sh build.sh
cd $ROOT

cd dao-factory
sh build.sh
cd $ROOT

cd ft-factory
sh build.sh
cd $ROOT

cd fungible-token
sh build.sh
cd $ROOT

