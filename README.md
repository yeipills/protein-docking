# cshape — formas de contexto de superficies proteicas

`cshape` calcula las **formas de contexto** (*context shapes*) de la superficie excluida al
solvente (SES) de una proteína, a partir de un archivo PDB (la superficie se genera
internamente, ver [docs/SURFACE.md](docs/SURFACE.md)) o de los archivos `.vert` / `.face` de MSMS.

Para cada **punto crítico** de la superficie se lanza una esfera de rayos; cada muestra de
cada rayo se clasifica en diez capas: la superficie (`ses`), cuatro capas interiores
(`in1`–`in4`), cuatro exteriores (`out1`–`out4`) y el volumen (`vol`). El resultado se puede
explorar en un visor 3D en el navegador y descargar en el formato de texto que usa el visor
de Unity.

Todo es un único binario en Rust (núcleo de cálculo, servidor HTTP y CLI) más un cliente web
estático.

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

Con `CSHAPE_TOKEN` definido, todas las rutas salvo `/api/health` exigen
`Authorization: Bearer <token>` (o `?token=` para descargas y EventSource).

Configuración del servidor (variables de entorno o banderas de `cshape serve --help`):
`CSHAPE_ADDR`, `CSHAPE_DATA`, `CSHAPE_WEB`, `CSHAPE_SAMPLES`, `CSHAPE_TOKEN`,
`CSHAPE_WORKERS`, `CSHAPE_MAX_UPLOAD_MB`.

## Estructura del repositorio

```
crates/cs-core/   núcleo: lectura MSMS/STL, BVH, distancia con signo, pipelines, exportadores
crates/cshape/    binario: CLI (run, compare, serve, health) y servidor HTTP
web/              cliente React + three.js
data/samples/     proteína de ejemplo 1AHW_l_u (.vert, .face, .stl)
tools/legacy-oracle/  vuelve a ejecutar los scripts originales para regenerar la referencia
docs/ALGORITHM.md el algoritmo, los defectos del original y cómo se valida
```

## Pruebas

```bash
make test     # Rust (incluye la comparación con los scripts originales) + web
make check    # rustfmt, clippy, tipos de TypeScript
make oracle   # vuelve a correr los scripts Python originales y compara (≈3 min)
```

## Historia

Este repositorio reúne tres intentos anteriores, que siguen disponibles en el historial de git:
los scripts originales de la tesis (eliminados en `4b5d28d`), la plataforma FastAPI + React +
Celery (v2.x, hasta `b99602a`) y dos reescrituras en Rust/Svelte que no compilaban y cuyo
docking era simulado (`514f7d4`). La versión 3 reemplaza a todos.

## Licencia

MIT — ver [LICENSE](LICENSE).
