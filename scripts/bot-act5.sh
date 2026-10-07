#!/usr/bin/env bash
# Every hero through Act 5 (from the Act 5 test start).
cd "$(dirname "$0")/.."
for h in sorceress vampire inventor valkyrie berserker reaper druid inquisitor; do
  bash scripts/bot-hero.sh "$h" 5
done
