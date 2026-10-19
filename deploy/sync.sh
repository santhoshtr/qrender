#!/bin/bash
# Build qrender-server as a static binary, copy it to the Toolforge tool
# "compose", restart the webservice and check that it answers.
#
# One-time setup, as the tool (ssh toolforge, then: become compose):
# Create a file named `service.template` with following content
# ```
# backend: kubernetes
# type: ruby3.1
# extra_args:
# - ./qrender-server
# ```
# Then:
#   toolforge envvars create USER_AGENT 'qrender/0.1 (https://compose.toolforge.org; sthottingal@wikimedia.org)'
#   toolforge envvars create REDIS_PREFIX "$(openssl rand -hex 16)"
#   toolforge webservice trixie start /data/project/compose/qrender-server
#
# Toolforge sets PORT (8000) and TOOL_REDIS_URI for the webservice. The
# Redis of Toolforge is shared and has no access control: keep REDIS_PREFIX
# secret.
set -euo pipefail
cd "$(dirname "$0")/.."

TARGET=x86_64-unknown-linux-musl
BIN=target/$TARGET/release/qrender-server

cargo build --release --locked --target "$TARGET" --bin qrender-server
strip "$BIN"
# rsync writes a temporary file and renames it: the running server never
# sees a partial file.
rsync -z "$BIN" toolforge:/data/project/compose/qrender-server
ssh toolforge 'become compose toolforge webservice restart'

for _ in $(seq 30); do
    curl -fsS https://compose.toolforge.org/healthz && echo && exit 0
    sleep 2
done
echo "qrender-server does not answer: see 'toolforge webservice logs'" >&2
exit 1
