#!/bin/bash
# A render started by an agent: ./render_agent.sh <pbrt options>
# (the same as pbrt's, for instance --input test_files/x.stage --output x.png --samples_ppx 256).
#
# pbrt's output goes to target/renders/<image>.log, not into a pipe: the progress table stays
# readable there while the render runs (Herdr's pbrt widget reads it from there), and the agent
# gets only its last state at the end, instead of the thousands of tables redrawn.
# The exit code is pbrt's (or the build's, when it fails).
# A render started by hand has no need to go through this.
set -u
cd "$(dirname "$0")"

cargo build --release --quiet || exit $?

output=output.png
args=("$@")
for ((i = 0; i < ${#args[@]} - 1; i++)); do
  [ "${args[i]}" = "--output" ] && output="${args[i + 1]}"
done
mkdir -p target/renders
log="target/renders/$(basename "${output%.*}").log"

./target/release/pbrt "$@" > "$log" 2>&1
code=$?

# The last table drawn (or, without a table, the last lines), escape sequences removed and
# carriage returns turned into line ends.
sed $'s/\x1b\\[[0-9;?]*[A-Za-z]//g' "$log" | tr '\r' '\n' | awk '
  /Rendering progress/ { n = 0; table = 1 }
  table { last[n++] = $0; next }
  NF { tail[t++ % 5] = $0 }
  END {
    if (n) for (i = 0; i < n; i++) print last[i]
    else for (i = (t > 5 ? t - 5 : 0); i < t; i++) print tail[i % 5]
  }'
echo "log: $log (code $code)"
exit $code
