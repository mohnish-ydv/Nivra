#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
sed -n '1,300p' "$ROOT/examples/d2/08_complete_architecture_tour.nva"
