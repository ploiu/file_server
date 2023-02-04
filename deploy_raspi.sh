#!/bin/bash

#
# adapted from https://medium.com/swlh/compiling-rust-for-raspberry-pi-arm-922b55dbb050
# some differences here:
#   Older versions of linux (such as raspbian) won't have glibc version we use, and by default rust will dynamically
#   link glibc. This will mean we can't run on raspi.
#   To get around this, you need to install the dev version of glibc (sudo apt install libc6-dev).
#
#   This script is meant for 64 bit OS for raspberry pi
#

set -o errexit
set -o nounset
set -o pipefail
set -o xtrace


#readonly TARGET_HOST=pi@raspberrypi
#readonly TARGET_PATH=/home/pi/file_server
readonly TARGET_ARCH=aarch64-unknown-linux-gnu
readonly SOURCE_PATH=./target/${TARGET_ARCH}/release/file_server

[[ "${TARGET_HOST}" = "" ]] && echo "missing TARGET_HOST" && exit
[[ "${TARGET_PATH}" = "" ]] && echo "missing TARGET_PATH" && exit
cargo build --release --target=${TARGET_ARCH}

rsync ${SOURCE_PATH} ${TARGET_HOST}:${TARGET_PATH}_exec
