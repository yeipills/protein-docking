#!/usr/bin/env bash
# Downloads the Protein-Protein Docking Benchmark 5.5 (Vreven et al., J Mol Biol 2015;
# https://zlab.wenglab.org/benchmark/) into data/benchmark/ (git-ignored):
#
#   data/benchmark/benchmark5.5/structures/XXXX_{r,l}_{u,b}.pdb
#   data/benchmark/Table_BM5.5.xlsx
#
# Usage: tools/benchmark/fetch.sh [destination]   (default: <repo>/data/benchmark)
#
# The case list derived from the table is versioned in tools/benchmark/cases.tsv; regenerate
# it with tools/benchmark/make_cases.py if the table changes.
set -euo pipefail

BASE_URL="https://zlab.wenglab.org/benchmark"
# Checksums of the files downloaded in October 2026, to notice upstream changes.
TGZ_SHA256="20c4df83308b1f0f4c3a04bf1d246059fd307da7a7c6431b386551687bea8841"
XLSX_SHA256="ee06580a716d74190982ca55626cb7982a9880d455251c9852b903b71501cb31"

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
dest="${1:-$repo/data/benchmark}"
mkdir -p "$dest"
cd "$dest"

fetch() { # url file sha256
    if [[ ! -s "$2" ]]; then
        echo "downloading $1"
        curl -fL --retry 3 -o "$2.part" "$1"
        mv "$2.part" "$2"
    fi
    local got
    got="$(sha256sum "$2" | cut -d' ' -f1)"
    if [[ "$got" != "$3" ]]; then
        echo "warning: $2 has sha256 $got, expected $3 (upstream file changed?)" >&2
    fi
}

fetch "$BASE_URL/benchmark5.5.tgz" benchmark5.5.tgz "$TGZ_SHA256"
fetch "$BASE_URL/Table_BM5.5.xlsx" Table_BM5.5.xlsx "$XLSX_SHA256"

if [[ ! -d benchmark5.5/structures ]]; then
    echo "extracting benchmark5.5.tgz"
    tar -xzf benchmark5.5.tgz
    # The archive was made on macOS: drop its AppleDouble "._*" companions.
    find benchmark5.5 -name '._*' -delete
fi

n=$(find benchmark5.5/structures -name '*.pdb' | wc -l)
echo "benchmark ready in $dest/benchmark5.5/structures ($n PDB files)"
