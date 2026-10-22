# Complejos de referencia

Estructuras del Protein-Protein Docking Benchmark 5.5 (Vreven et al., J Mol Biol 2015;
https://zlab.wenglab.org/benchmark/), que a su vez provienen del PDB.

Por cada complejo `XXXX`:

- `XXXX_r_u.pdb`, `XXXX_l_u.pdb`: receptor y ligando **no unidos** (entrada del docking).
  En el benchmark ya están superpuestos sobre el complejo, así que hay que aleatorizar la
  posición del ligando antes de acoplar para no sesgar el resultado.
- `XXXX_r_b.pdb`, `XXXX_l_b.pdb`: receptor y ligando **unidos** (la solución, para evaluar).

`1AHW`: Fab 5G9 (receptor) con factor tisular (ligando). La superficie MSMS de
`1AHW_l_u` es la muestra de `data/samples/1AHW_l_u`.

El benchmark completo se descarga con `tools/benchmark/fetch.sh` (no se versiona).
