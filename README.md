# cshape — formas de contexto y docking de proteínas

`cshape` calcula las **formas de contexto** (*context shapes*) de la superficie excluida al
solvente (SES) de una proteína, a partir de un archivo PDB (la superficie se genera
internamente, ver [docs/SURFACE.md](docs/SURFACE.md)) o de los archivos `.vert` / `.face` de MSMS.

Para cada **punto crítico** de la superficie se lanza una esfera de rayos; cada muestra de
cada rayo se clasifica en diez capas: la superficie (`ses`), cuatro capas interiores
(`in1`–`in4`), cuatro exteriores (`out1`–`out4`) y el volumen (`vol`). El resultado se puede
explorar en un visor 3D en el navegador y descargar en el formato de texto que usa el visor
de Unity.

Sobre esas formas, `cshape` también hace **docking rígido proteína–proteína**: busca
parches de superficie complementarios entre un receptor y un ligando, puntúa las poses con
los átomos y, si se conoce el complejo real, las califica con los criterios estándar
**CAPRI y DockQ** (validados contra la implementación oficial).

Todo es un único binario en Rust (núcleo de cálculo, servidor HTTP y CLI) más un cliente web
estático.

| Documento | Contenido |
|---|---|
| [docs/ALGORITHM.md](docs/ALGORITHM.md) | Formas de contexto, defectos del original y validación |
| [docs/SURFACE.md](docs/SURFACE.md) | Generador de superficie excluida al solvente desde PDB |
| [docs/DOCKING.md](docs/DOCKING.md) | Motor de docking, parámetros y resultados |
| [docs/EVALUATION.md](docs/EVALUATION.md) | CAPRI/DockQ y benchmark Docking Benchmark 5.5 |

## Dos modos de cálculo

| | **Corregido** (por defecto) | **Fiel al original** |
|---|---|---|
| Objetivo | Geometría correcta | Mismos números que los scripts de la tesis |
| Entrada | `.vert` + `.face` | `.vert` + `.face` + `.stl` |
| Índices MSMS | 1-based, correctos | usados como 0-based (centroides desplazados) |
| Rayos | Fibonacci, uniformes, configurables | `phi, theta ∈ linspace(0, 2π, 10)`: 100 rayos, muchos repetidos |
| Dentro / fuera | distancia con signo exacta a la malla SES | `intersects_any` de trimesh con la posición absoluta como dirección |
| Capas | bandas de distancia a la superficie | desplazamientos radiales desde el origen de coordenadas |
| Puntos críticos | separados al menos `min_spacing` Å | solo se descartan los 100 vecinos más cercanos |
| Validación | pruebas geométricas contra fuerza bruta | **idéntico byte a byte** a los 24 archivos de los scripts originales |

El modo fiel existe para comparar y justificar el cambio. Los defectos que reproduce están
descritos en [docs/ALGORITHM.md](docs/ALGORITHM.md).

## Rendimiento

Proteína de ejemplo `1AHW_l_u` (11.141 vértices, 22.290 caras), 4 núcleos:

| | Tiempo de cálculo |
|---|---|
| Scripts originales en Python (Script03 + 04 + 05)¹ | 141 s |
| `cshape`, modo fiel (mismos resultados) | 0,3 s |
| `cshape`, modo corregido (valores por defecto) | 0,08 s |
| `cshape`, modo corregido con 2.092 formas × 200 rayos × 32 muestras (13,4 M distancias exactas) | 3,8 s |

¹ Con Script04 ya optimizado (búsquedas por diccionario en vez de recorrer listas completas);
tal como estaba escrito tardaba del orden de una hora.

## Docking

Docking Benchmark 5.5, categoría rígida completa (162 complejos), ligando en posición
aleatoria, % de casos con al menos una pose de la calidad indicada (aceptable / media / alta):

| | Top 1 | Top 10 | Top 100 | Tiempo medio |
|---|---|---|---|---|
| `cshape` (superficie de átomos) | 4 / 2 / 0 | 20 / 8 / 1 | 43 / 15 / 1 | 9,3 s |
| Poses al azar (referencia) | 0 / 0 / 0 | — | 3 / 0 / 0 | — |
| Techo rígido (mejor colocación posible) | 100 / 99 / 69 | | | |

Es una línea base clásica de complementariedad de forma: rápida y bien medida, pero lejos
de los métodos actuales. El mayor margen de mejora está en la puntuación (las poses casi
nativas suelen generarse pero no quedan arriba). Los detalles y limitaciones, en
[docs/DOCKING.md](docs/DOCKING.md).

```bash
make dock     # acopla 1AHW (Fab + factor tisular) y califica cada pose contra el cristal
make bench    # descarga el benchmark y corre la categoría rígida (~25 min)
```

## Inicio rápido

### Docker

```bash
docker compose up --build
# http://localhost:8080
```

Para exigir un token de acceso: `CSHAPE_TOKEN=un-secreto docker compose up`.

### Desde el código

Requiere Rust ≥ 1.85 y Node ≥ 20.

```bash
make build                        # cliente web + binario release
./target/release/cshape serve     # http://localhost:8080
```

En desarrollo, `make dev` levanta el servidor y Vite (`http://localhost:5173`, con proxy a la API).

### Línea de comandos

```bash
cshape run --pdb prot.pdb --out resultados/                          # genera la SES
cshape run --pdb prot.pdb --surface ses.json --out resultados/       # {"probe": 1.4, "grid_spacing": 0.5, ...}
cshape run --vert prot.vert --face prot.face --out resultados/
cshape run --mode legacy --vert prot.vert --face prot.face --stl prot.stl --out legacy/
cshape run --vert prot.vert --face prot.face --params params.json   # parámetros propios
cshape compare dirA/ dirB/        # compara línea a línea los .txt comunes

# Docking: poses ordenadas, PDB de los mejores complejos y, con el complejo real, CAPRI/DockQ
cshape dock --receptor rec.pdb --ligand lig.pdb --out dock/
cshape dock --receptor rec_u.pdb --ligand lig_u.pdb \
    --native-receptor rec_b.pdb --native-ligand lig_b.pdb --randomize-seed 1 --out dock/
cshape bench --category rigid --limit 20 --out bench/   # Docking Benchmark 5.5
```

## Parámetros (modo corregido)

| Parámetro | Defecto | Descripción |
|---|---|---|
| `min_spacing` | `10.0` Å | Distancia mínima entre puntos críticos |
| `radius` | `3.0` Å | Largo de cada rayo (radio de la forma) |
| `rays` | `100` | Rayos por forma |
| `samples` | `32` | Muestras por rayo |
| `layer_edges` | `[-1, -0.8, -0.4, -0.2, 0, 0.2, 0.4, 0.8, 1]` Å | Bordes de `in4 … in1 \| out1 … out4` |
| `ses_half_width` | media separación entre muestras | Media anchura de la banda `ses` |
| `skip_face_types` | `[1]` | Tipos de cara MSMS que no aportan puntos de superficie |

Con `min_spacing` = 10 Å resultan menos formas que en el modo fiel (49 frente a 160 en el
ejemplo), porque el original solo descartaba los 100 vecinos más cercanos y en zonas densas
quedaban puntos a menos de 10 Å. Para una densidad parecida, use unos 6 Å.

## Archivos de salida

| Archivo | Contenido |
|---|---|
| `<nombre>_cs_<capa>_unity.txt` (×10) | Una línea por rayo: `forma rayo v₁ … vₙ cx cy cz ex ey ez` |
| `<nombre>_resumen_cs_unity.txt` | Centro de cada forma |
| `summary.json` | Conteos, ocupación por capa, parámetros y tiempos |
| `viewer.bin` | Malla y formas para el visor web (formato en `crates/cs-core/src/export.rs`) |
| *(entrada PDB)* `<nombre>.vert`, `<nombre>.face` | Superficie generada, en formato MSMS |
| *(modo fiel)* `centroidesPCS.txt`, `<nombre>_rayos_contexto.txt`, `<nombre>_cr_totales.txt` | Salidas de Script02 y Script03 |
| *(modo fiel, opcional)* `<nombre>_<capa>.txt` (×10) | Archivos intermedios de Script04 |
| *(docking)* `poses.json` | Poses ordenadas: transformación del ligando, puntaje, componentes y, si hay complejo real, métricas CAPRI/DockQ |
| *(docking)* `<nombre>_pose_NNN.pdb` | Complejo receptor + ligando acoplado de las mejores poses |
| *(docking)* `dock_viewer.bin` | Superficies del receptor y del ligando para el visor |

## API

| Método y ruta | |
|---|---|
| `GET /api/health` | Estado (sin autenticación) |
| `GET /api/config` | Versión y parámetros por defecto |
| `GET /api/jobs` · `POST /api/jobs` | Listar · crear (multipart: `pdb` **o** `vert` + `face`, `stl?`, `name?`, `mode?`, `params?` JSON, `surface?` JSON, `legacy_intermediate?`) |
| `GET /api/jobs/{id}` · `DELETE /api/jobs/{id}` | Detalle con resumen · borrar |
| `POST /api/jobs/{id}/cancel` | Cancelar |
| `GET /api/jobs/{id}/events` | Progreso en vivo (Server-Sent Events) |
| `GET /api/jobs/{id}/files/{archivo}` · `GET /api/jobs/{id}/archive` | Un archivo · todo en ZIP |
| `GET /api/samples` · `POST /api/samples/{nombre}/jobs` | Muestras incluidas · calcular una |
| `POST /api/dock` | Docking (multipart: `receptor`, `ligand`, `native_receptor?` + `native_ligand?`, `surface?` = `atoms`\|`ses`, `params?` JSON, `randomize_seed?`, `complexes?`, `name?`) |
| `GET /api/complexes` · `POST /api/complexes/{nombre}/dock` | Complejos incluidos · acoplar uno (con evaluación si trae el complejo unido) |

Con `CSHAPE_TOKEN` definido, todas las rutas salvo `/api/health` exigen
`Authorization: Bearer <token>` (o `?token=` para descargas y EventSource).

Configuración del servidor (variables de entorno o banderas de `cshape serve --help`):
`CSHAPE_ADDR`, `CSHAPE_DATA`, `CSHAPE_WEB`, `CSHAPE_SAMPLES`, `CSHAPE_COMPLEXES`, `CSHAPE_TOKEN`,
`CSHAPE_WORKERS`, `CSHAPE_MAX_UPLOAD_MB`.

## Estructura del repositorio

```
crates/cs-core/   núcleo: PDB/MSMS/STL, superficie SES, formas de contexto, docking, evaluación
crates/cshape/    binario: CLI (run, dock, bench, compare, serve, health) y servidor HTTP
web/              cliente React + three.js
data/samples/     proteína de ejemplo 1AHW_l_u (.vert, .face, .stl)
data/complexes/   complejo 1AHW del Docking Benchmark (receptor y ligando, libres y unidos)
tools/legacy-oracle/  vuelve a ejecutar los scripts originales para regenerar la referencia
tools/benchmark/  descarga del Docking Benchmark 5.5 y tabla de casos
docs/             algoritmo, superficie, docking y evaluación
```

## Pruebas

```bash
make test     # Rust (incluye la comparación con los scripts originales) + web
make check    # rustfmt, clippy, tipos de TypeScript
make oracle   # vuelve a correr los scripts Python originales y compara (≈3 min)
make dock     # docking de 1AHW con evaluación CAPRI/DockQ (~12 s)
```

## Historia

Este repositorio reúne tres intentos anteriores, que siguen disponibles en el historial de git:
los scripts originales de la tesis (eliminados en `4b5d28d`), la plataforma FastAPI + React +
Celery (v2.x, hasta `b99602a`) y dos reescrituras en Rust/Svelte que no compilaban y cuyo
docking era simulado (`514f7d4`). La versión 3 reemplaza a todos.

## Licencia

MIT — ver [LICENSE](LICENSE).
