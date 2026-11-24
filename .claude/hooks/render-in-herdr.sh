#!/bin/bash
# PreToolUse (Bash): in a Herdr terminal, a render started by an agent goes through
# ./render_agent.sh (see CLAUDE.md, « Lancer un rendu »), which leaves its progress readable by
# Herdr's pbrt widget. Outside Herdr (no HC_TERMINAL_ID), nothing is checked: nobody reads the
# progress there.
[ -n "${HC_TERMINAL_ID:-}" ] || exit 0

command=$(jq -r '.tool_input.command // empty')
case "$command" in *render_agent.sh*) exit 0 ;; esac

# pbrt started directly: its binary, or `cargo run` without --bin (or with --bin pbrt).
# `cargo run --bin bvh_stats` and the other tools stay free, and so does --help.
direct=0
# The binary in command position (first, or after ; & | ( or time / nice / env).
if grep -Eq '(^|[;&|(])[[:space:]]*((time|nice|env)[[:space:]]+)*(\./)?target/(release|debug)/pbrt([[:space:]]|$)' <<< "$command"; then
  direct=1
elif grep -Eq 'cargo[[:space:]]+run' <<< "$command"; then
  bin=$(sed -nE 's/.*--(bin|example)[[:space:]=]+([^[:space:]]+).*/\2/p' <<< "$command")
  [ -z "$bin" ] || [ "$bin" = pbrt ] && direct=1
fi
[ "$direct" = 1 ] || exit 0
grep -Eq -- '(^|[[:space:]])(--help|-h)([[:space:]]|$)' <<< "$command" && exit 0

jq -n '{hookSpecificOutput: {hookEventName: "PreToolUse", permissionDecision: "deny",
  permissionDecisionReason: "In Herdr, a render goes through ./render_agent.sh <pbrt options> (the same options, without | tail): its progress stays readable by the pbrt widget, and only its last state comes back. See CLAUDE.md, « Lancer un rendu »."}}'
