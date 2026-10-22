#!/usr/bin/env bash
# Re-runs the original thesis scripts (Script02-05) on the bundled 1AHW_l_u sample, to
# regenerate the reference outputs that legacy mode is tested against.
#
# The scripts are taken from git history (they were removed from the tree in 4b5d28d).
# Only file paths are changed, plus one equivalent rewrite in Script04 that replaces
# quadratic list scans with dictionary lookups (same order, same values) so it finishes
# in seconds instead of hours. See patch.py.
#
# Usage: tools/legacy-oracle/run.sh <output-dir>
set -euo pipefail

out=$(realpath -m "${1:?usage: run.sh <output-dir>}")
repo=$(git rev-parse --show-toplevel)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

src='Backend/C-lculos-Previos-main/Centroides de triangulos/Programa_python'
git -C "$repo" archive 4b5d28d^ "$src" | tar -x -C "$work"
cd "$work/$src"

# The archived folders also hold old outputs; start from empty ones.
rm -rf proteinas proteinas_stl proteinas_cr proteinas_cs proteinas_cs_unity
mkdir -p proteinas proteinas_stl proteinas_cr proteinas_cs proteinas_cs_unity
cp "$repo/data/samples/1AHW_l_u/1AHW_l_u.vert" "$repo/data/samples/1AHW_l_u/1AHW_l_u.face" proteinas/
cp "$repo/data/samples/1AHW_l_u/1AHW_l_u.stl" proteinas_stl/

python3 -m venv "$work/venv"
"$work/venv/bin/pip" install -q numpy scipy trimesh rtree cython setuptools
"$work/venv/bin/python" "$repo/tools/legacy-oracle/patch.py" .
"$work/venv/bin/cythonize" -q -i Script04.pyx
"$work/venv/bin/python" run_oracle.py > /dev/null

mkdir -p "$out"
cp -f centroidesPCS.txt proteinas_cr/1AHW_l_u_* proteinas_cs/1AHW_l_u_* proteinas_cs_unity/1AHW_l_u_* "$out/"
echo "oracle outputs written to $out"
