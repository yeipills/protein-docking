"""Minimal changes needed to run the original scripts headless on the sample.

1. Script04 compares every SES point against whole per-layer lists
   (`[i[2] for i in cs_x if i[1] == number_ray]`), which is quadratic. The lists are
   grouped by ray once and looked up instead; order and values are unchanged.
2. Script04 wrote its per-layer files to '../proteinas_cs_unity<name>', but Script05 reads
   './proteinas_cs/<name>'. The output folder is pointed where Script05 looks.
3. run_oracle.py calls Script03, Script04 and Script05 in sequence for 1AHW_l_u.
"""
import pathlib
import sys

root = pathlib.Path(sys.argv[1])
s4 = root / "Script04_evaluacion_capas.py"
src = s4.read_text()

lists = ["ses", "in1", "in2", "in3", "in4", "out1", "out2", "out3", "out4"]
for name in lists:
    old = f"{name} = [i[2] for i in cs_{name} if i[1] == number_ray]"
    assert old in src, old
    src = src.replace(old, f"{name} = G['{name}'].get(number_ray, [])")

anchor = "    cont = 0\n    for item in SES_points:\n"
assert anchor in src
grouping = "    G = {}\n"
for name in lists:
    grouping += (
        f"    G['{name}'] = {{}}\n"
        f"    for _i in cs_{name}:\n"
        f"        G['{name}'].setdefault(_i[1], []).append(_i[2])\n"
    )
src = src.replace(anchor, grouping + anchor, 1)

old_dir = "carpeta_cs = str('../proteinas_cs_unity')"
assert old_dir in src
src = src.replace(old_dir, "carpeta_cs = str('./proteinas_cs/')")
s4.write_text(src)

(root / "run_oracle.py").write_text(
    """import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
os.chdir(os.path.dirname(os.path.abspath(__file__)))
import Script03_rayos_contexto as S3
S3.getCR('1AHW_l_u', '1AHW_l_u.vert', '1AHW_l_u.face')
import Script04_evaluacion_capas as S4
S4.getCs('1AHW_l_u', '1AHW_l_u_cr_totales.txt', '1AHW_l_u_rayos_contexto.txt')
import Script05_preparacion_capas_unity
"""
)
