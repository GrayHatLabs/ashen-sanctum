#!/usr/bin/env bash
# A long bot run that should finish whole acts (kept alive): scripts/bot-long.sh <act> <hero> [minutes]
cd "$(dirname "$0")/.."
ASHEN_ACT=$1 ASHEN_CLASS=$2 ASHEN_MINUTES=${3:-90} ASHEN_GOD=1 "$HOME/.cache/ashensanctum-target/release/ashensanctum" --selftest 2>&1 \
  | grep -E "boss down|selftest|panicked" | sed -E 's/ stats=.*//; s/ \(token[^)]*\)//'
