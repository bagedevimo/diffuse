#!/bin/bash

set -e

cargo build

if [ "$1" == "--notrace" ]; then
	git push local --all
else
	GIT_TRACE=2 GIT_CURL_VERBOSE=2 GIT_TRACE_PERFORMANCE=2 GIT_TRACE_PACK_ACCESS=2 GIT_TRACE_PACKET=2 GIT_TRACE_PACKFILE=2 GIT_TRACE_SETUP=2 GIT_TRACE_SHALLOW=2 git push --receive-pack=./target/debug/diffuse_recv_pack local --all
fi

head -n 10 out.log
