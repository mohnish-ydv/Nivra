#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
sed -n '1,260p' "$ROOT/examples/design/05_complete_tour.trn"
