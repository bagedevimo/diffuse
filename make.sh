#!/bin/bash

set -e

cargo build
sudo cp target/debug/diffuse_recv_pack /Users/git/ben-git/git-receive-pack && sudo chown git /Users/git/ben-git/git-receive-pack

GIT_TRACE=2 GIT_CURL_VERBOSE=2 GIT_TRACE_PERFORMANCE=2 GIT_TRACE_PACK_ACCESS=2 GIT_TRACE_PACKET=2 GIT_TRACE_PACKFILE=2 GIT_TRACE_SETUP=2 GIT_TRACE_SHALLOW=2 git push local master

sudo head -n 5 /Users/git/out.log