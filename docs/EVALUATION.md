# Evaluación del docking

El módulo `cs_core::eval` mide la calidad de las poses de un docking proteína–proteína con los
criterios estándar de CAPRI y con DockQ, y ejecuta un docking cualquiera sobre el
Protein-Protein Docking Benchmark 5.5 (BM5.5) para obtener tasas de éxito comparables con la
literatura.

- `eval::Native`: complejo nativo preparado para evaluar miles de poses.
- `eval::align`: correspondencia de residuos entre estructuras no unidas y unidas.
- `eval::bench`: lista de casos, aleatorización, ejecución y agregación de resultados.

## Métricas

Una **pose** es la transformación rígida (`Rigid`) que se aplica al ligando no unido, tal
como está en su archivo, para colocarlo sobre el receptor no unido, que no se mueve. Se
compara el complejo resultante con el complejo nativo (receptor y ligando unidos,
`XXXX_r_b.pdb` + `XXXX_l_b.pdb`). Solo se usan átomos pesados de residuos de proteína.

### Correspondencia de residuos

Entre la estructura no unida y la unida suelen cambiar los identificadores de cadena, la
numeración y los residuos que faltan (bucles desordenados, extremos). Por eso los residuos se
emparejan por secuencia:

1. Cada cadena unida se alinea con cada cadena no unida (Needleman–Wunsch global con huecos
   afines: coincidencia 5, sustitución 0, apertura −4, extensión −0,5, huecos de los extremos
   penalizados; son los parámetros de DockQ).
2. Solo se emparejan residuos **idénticos** (las mutaciones quedan fuera, como en DockQ). Un
   par de cadenas cuenta si empareja al menos la mitad de la cadena más corta.
3. Las cadenas se asignan uno a uno maximizando el número de residuos emparejados. Pueden
   sobrar cadenas no unidas (p. ej. el trímero completo de hemaglutinina en 2VIS, o las dos
   copias del ligando en 1FC2).
4. Si hay copias idénticas de una cadena, varias asignaciones son igual de buenas. Se
   conservan todas (hasta 16 combinaciones receptor × ligando) y cada pose se puntúa con la
   que dé mejor DockQ: unirse a cualquiera de las copias de un homodímero es igual de
   correcto. DockQ v2 hace lo mismo al permutar cadenas.

`NativeInfo` informa del emparejamiento elegido, de los residuos emparejados y de cuántos
contactos nativos pueden reproducirse con las estructuras no unidas.

### Definiciones

| Métrica | Definición |
|---|---|
| **Contacto** | Par residuo del receptor – residuo del ligando con algún par de átomos pesados a menos de 5 Å. |
| **fnat** | Contactos nativos presentes en la pose / contactos nativos. El denominador cuenta **todos** los contactos del complejo unido, también los de residuos que faltan en las estructuras no unidas (que, por tanto, nunca se reproducen). |
| **fnonnat** | Contactos de la pose que no son nativos / contactos de la pose (entre residuos emparejados). |
| **Interfaz** | Residuos emparejados con algún átomo pesado a menos de 10 Å de un residuo emparejado de la otra proteína, en el complejo nativo. |
| **iRMSD** | RMSD de los átomos N, CA, C, O de los residuos de la interfaz (de ambas proteínas) tras la superposición óptima (Kabsch) sobre el nativo. |
| **LRMSD** | RMSD de los átomos N, CA, C, O de la proteína más pequeña tras superponer la más grande (todos sus residuos emparejados). Con el convenio de CAPRI la "grande" es la que tiene más residuos en el nativo: casi siempre el receptor, pero no en casos como 1B6C o 1FC2. |
| **DockQ** | (fnat + 1/(1 + (iRMSD/1,5)²) + 1/(1 + (LRMSD/8,5)²)) / 3, en [0, 1] (Basu y Wallner, 2016). |

Clases de CAPRI (Méndez et al. 2003; Lensink et al. 2007): la mejor clase cuyo umbral de
fnat se cumpla junto con **alguno** de sus dos umbrales de RMSD.

| Clase | fnat | | LRMSD | | iRMSD |
|---|---|---|---|---|---|
| high | ≥ 0,5 | y | ≤ 1,0 Å | o | ≤ 1,0 Å |
| medium | ≥ 0,3 | y | ≤ 5,0 Å | o | ≤ 2,0 Å |
| acceptable | ≥ 0,1 | y | ≤ 10,0 Å | o | ≤ 4,0 Å |
| incorrect | resto | | | | |

Los umbrales de DockQ (0,23 / 0,49 / 0,80) aproximan las mismas clases, pero la clase que se
informa es la de CAPRI.

### Rendimiento

`Native::new` precalcula todo lo que no depende de la pose: alineamientos, contactos nativos,
átomos de la interfaz y la superposición del receptor (que no se mueve, así que para LRMSD
basta componer esa superposición con la pose). Evaluar una pose consiste en transformar los
residuos del ligando cercanos al receptor, buscar sus contactos en una rejilla y hacer una
superposición pequeña: entre 0,1 y 0,3 ms por pose y núcleo en complejos del BM5.5 (más en
poses que solapan mucho con el receptor). `Native::evaluate_many` reparte las poses entre
todos los núcleos. Preparar un caso cuesta entre 5 y 30 ms.

### Pose de referencia

`Native::optimal_pose()` coloca el ligando no unido en el modo de unión nativo: se superpone
la cadena principal de la interfaz de cada proteína sobre la del nativo y se encadenan ambas
superposiciones. Sus métricas aproximan el techo de un docking rígido con esas estructuras
no unidas. Se usa la interfaz y no la proteína entera porque los movimientos lejos de la
interfaz desplazarían el sitio de unión. Por ejemplo, en el Fab de 6B0S el codo cambia entre
la forma unida y la no unida (RMSD del Fv 0,95 Å, del Fab entero 10,8 Å).

## Validación con DockQ oficial

Se comparó con DockQ v2.1.3 (`pip install DockQ`, Mirabello y Wallner 2024) en 9 complejos
del BM5.5 que cubren los casos difíciles para la correspondencia de residuos:

| Caso | Por qué |
|---|---|
| 1AHW | Receptor de dos cadenas (Fab L/H → A/B), un residuo ausente en el no unido |
| BOYV | Cadenas renombradas (B → A), ligando con residuos ausentes |
| 1FC2 | Cadena sin identificador, dos copias idénticas del ligando no unido, ligando mayor que el receptor |
| 2VIS | Tres copias de hemaglutinina en el no unido para una cadena unida |
| 3P57 | Receptor homodimérico (dos cadenas idénticas) |
| 1ATN | Difícil (cambio conformacional), cadena sin identificador |
| 1B6C | El ligando (326 residuos) es mayor que el receptor (107): LRMSD superpone el ligando |
| 1IRA | Difícil, cadenas con identificadores numéricos |
| 6B0S | Fab con cambio de codo |

Para cada complejo, `examples/dockq_poses.rs` genera 11 poses: la del archivo (ya superpuesta
en el benchmark), la de referencia, siete perturbaciones crecientes (de 3° y 0,5 Å a 90° y
14 Å) y dos poses aleatorias. En total son 99 poses que cubren todas las clases (13 high,
13 medium, 20 acceptable y 53 incorrect). Escribe cada modelo y el nativo en PDB y
`tools/benchmark/validate_dockq.py` ejecuta DockQ sobre cada par.

DockQ v2 puntúa cada par de cadenas por separado, mientras que en el benchmark (y en CAPRI)
cada proteína es un cuerpo. Por eso los archivos de validación fusionan las cadenas del
receptor en la cadena A y las del ligando en la B, renumeradas de forma consecutiva, y se
ejecuta DockQ con `--mapping AB:AB`. Solo se escriben los residuos que usa nuestro evaluador,
así que ambos programas ven los mismos átomos.

**Resultado:** coinciden todas las métricas en las 99 poses.

| Caso | Poses | DockQ (rango) | máx. \|Δfnat\| | máx. \|ΔiRMSD\| | máx. \|ΔLRMSD\| | máx. \|ΔDockQ\| |
|---|---|---|---|---|---|---|
| 1AHW | 11 | 0,012 – 0,834 | < 1e-6 | < 1e-5 Å | < 1e-4 Å | < 1e-5 |
| BOYV | 11 | 0,019 – 0,823 | < 1e-6 | < 1e-5 Å | < 1e-4 Å | < 1e-5 |
| 1FC2 | 11 | 0,006 – 0,616 | < 1e-6 | < 1e-5 Å | < 1e-4 Å | < 1e-5 |
| 2VIS | 11 | 0,012 – 0,499 | < 1e-6 | < 1e-5 Å | < 1e-4 Å | < 1e-5 |
| 3P57 | 11 | 0,015 – 0,830 | < 1e-6 | < 1e-5 Å | < 1e-4 Å | < 1e-5 |
| 1ATN | 11 | 0,012 – 0,497 | < 1e-6 | < 1e-5 Å | < 1e-4 Å | < 1e-5 |
| 1B6C | 11 | 0,006 – 0,708 | < 1e-6 | < 1e-5 Å | < 1e-4 Å | < 1e-5 |
| 1IRA | 11 | 0,020 – 0,381 | < 1e-6 | < 1e-5 Å | < 1e-4 Å | < 1e-5 |
| 6B0S | 11 | 0,018 – 0,596 | < 1e-6 | < 1e-5 Å | < 1e-4 Å | < 1e-5 |

Las diferencias máximas globales son 5e-7 en fnat y fnonnat, 6e-6 Å en iRMSD, 7e-5 Å en
LRMSD y 1e-6 en DockQ: es la precisión simple (`float32`) con que calcula DockQ.

Esta coincidencia exacta exige comparar lo mismo, y hay dos detalles que conviene conocer:

- **Redondeo de coordenadas.** DockQ lee las poses de un PDB, con coordenadas redondeadas a
  0,001 Å. Un par de residuos a 4,9996 Å puede quedar a 5,0003 Å y cambiar fnat en un
  contacto. En una versión anterior del conjunto de validación pasó una vez en 99 poses
  (1B6C: fnat 0,536 frente a 0,518, DockQ 0,454 frente a 0,448). Por eso la tabla compara
  nuestras métricas calculadas sobre las mismas coordenadas redondeadas. Sobre las
  coordenadas exactas, la tolerancia razonable es de ±1 contacto en fnat, ±0,005 Å en las RMSD
  y ±0,01 en DockQ.
- **Copias idénticas de cadenas.** Los archivos fusionados fijan una asignación de cadenas.
  `Native::evaluate` informa de la mejor entre las alternativas (ver arriba), así que en
  poses alejadas del nativo de 1FC2 y 2VIS puede dar un DockQ algo mayor (p. ej. 0,070 frente
  a 0,022 en una pose de 2VIS que toca otra copia de la hemaglutinina). La validación usa
  `Native::evaluate_each` con la misma asignación que se escribió.

`crates/cs-core/tests/eval.rs` repite la comparación con `tests/data/dockq_reference.tsv`
(las poses y los valores de DockQ, sin incluir DockQ). 1AHW se comprueba siempre. Los otros
ocho casos se comprueban si el benchmark está descargado.

Para repetir la validación:

```bash
tools/benchmark/fetch.sh
python3 -m venv /tmp/dockq && /tmp/dockq/bin/pip install DockQ
S=data/benchmark/benchmark5.5/structures
cargo run --release -p cs-core --example dockq_poses -- /tmp/val data/complexes/1AHW/1AHW \
    $S/BOYV $S/1FC2 $S/2VIS $S/3P57 $S/1ATN $S/1B6C $S/1IRA $S/6B0S
python3 tools/benchmark/validate_dockq.py /tmp/val --dockq /tmp/dockq/bin/DockQ \
    --fixture crates/cs-core/tests/data/dockq_reference.tsv
```

### Diferencias con DockQ fuera de la validación

- Los residuos modificados escritos como HETATM (MSE, SEP, TPO, PTR, LLP…) cuentan como
  residuos de proteína con la letra de su aminoácido de origen. DockQ ignora todos los
  HETATM. Ocurre en pocos archivos del BM5.5 y solo cambia algún contacto.
- Con ubicaciones alternativas se usa la primera (en blanco o `A`). Biopython elige la de
  mayor ocupación.
- Si hay empates de puntuación, el alineamiento puede elegir otro de los alineamientos
  óptimos. Solo afecta a residuos repetidos junto a un hueco.

## Evaluar un modelo

```bash
# Receptor y ligando en archivos separados (p. ej. receptor no unido + ligando acoplado)
cargo run --release -p cs-core --example evaluate -- \
    nativo_r_b.pdb nativo_l_b.pdb receptor.pdb ligando.pdb
# Un complejo, separando el ligando por cadenas; --json para salida JSON
cargo run --release -p cs-core --example evaluate -- \
    nativo_r_b.pdb nativo_l_b.pdb modelo.pdb --ligand-chains C --json
```

Desde Rust:

```rust
use cs_core::eval::Native;

let native = Native::new(&r_b, &l_b, &r_u, &l_u)?; // una vez por complejo
let m = native.evaluate(&pose);                    // pose: Rigid aplicada a l_u
println!("{} DockQ {:.3}", m.capri.name(), m.dockq);
let todas = native.evaluate_many(&poses);          // en paralelo
```

Si el modelo ya tiene el ligando en su sitio, se pasa como "no unido" y se evalúa la
identidad: `Native::new(&r_b, &l_b, &modelo_r, &modelo_l)?.evaluate(&Rigid::IDENTITY)`.

## Benchmark

### Datos

```bash
tools/benchmark/fetch.sh   # descarga en data/benchmark/ (ignorado por git)
```

`tools/benchmark/cases.tsv` es la tabla de casos (generada desde `Table_BM5.5.xlsx` con
`tools/benchmark/make_cases.py`), con 257 casos: 162 rígidos, 60 de dificultad media y 35
difíciles. Columnas: `id` (prefijo de los archivos), `complex` (código y cadenas, p. ej.
`1AHW_AB:C`), `category` (`rigid`, `medium`, `difficult`), `type` (AA, AS, EI, ES, ER, OG,
OR, OX), `irmsd` (I-RMSD no unido–unido, Å), `dasa` (superficie enterrada, Å²), `version`
(versión del benchmark en que entró el caso) y entradas y nombres de las proteínas no unidas.

Los complejos que aparecen varias veces en la tabla con distintas cadenas tienen archivos con
otro prefijo: 1OYV_B:I → `BOYV`, 3AAD_A:D → `BAAD`, 3P57_CD:P → `BP57`, 3P57_IJ:P →
`CP57`. Para 1JPS y 1FAK se usan los `XXXX_l_u.pdb` oficiales, no las variantes `.pdb.2`
(ver el README del benchmark). La descarga trae además 14 complejos que no figuran en la tabla de
la 5.5 (1BGX, 1QFW, 2HMI…). No se usan.

### El arnés

```rust
use cs_core::eval::bench::{self, BenchConfig, DockInput};

let cases = bench::load_cases(Path::new("tools/benchmark/cases.tsv"))?;
let dir = Path::new("data/benchmark/benchmark5.5/structures");
let docker = |receptor: &Structure, ligand: &Structure, input: &DockInput| -> Vec<Rigid> {
    // poses del `ligand` recibido, mejor primero (el receptor no se mueve)
};
let report = bench::run("mi-docker", &cases, dir, &BenchConfig::default(), docker, &NoProgress)?;
std::fs::write("report.json", report.to_json())?;
std::fs::write("report.md", report.to_markdown())?;
```

Para cada caso:

1. Lee las cuatro estructuras y prepara el `Native`.
2. **Aleatoriza el ligando no unido**: le aplica una rotación uniforme alrededor de su
   centroide y coloca el centroide en una dirección aleatoria desde el centroide del receptor,
   con las esferas envolventes separadas `start_gap` Å (10 por defecto). En el benchmark los
   no unidos vienen superpuestos sobre el complejo, así que acoplarlos tal cual filtraría la
   solución a cualquier método sensible a la posición inicial. La semilla de cada caso depende
   de la semilla global y del identificador, no del orden de los casos.
3. Llama al docker con el receptor no unido (sin mover) y el ligando aleatorizado.
   `DockInput` trae los metadatos del caso (nunca las estructuras unidas) y una semilla para
   dockers estocásticos.
4. **Compone las transformaciones**: el docker devuelve poses `D` del ligando que recibió,
   que es `R(l_u)`. La pose respecto al archivo original es `D ∘ R`, y es la que se evalúa
   (las `n` primeras, `top_n` = 100 por defecto).
5. Guarda por caso el primer rango que alcanza cada clase, la mejor pose por DockQ y su
   rango, las métricas del top-1, el techo rígido (`optimal_pose`) y los tiempos de lectura,
   preparación, docking y evaluación. Los errores (archivos ausentes, sin interfaz…) se
   registran en el caso y cuentan como fallo.
6. Agrega por categoría y en total la **tasa de éxito** en top-1, 5, 10 y 100: la fracción
   de casos con alguna pose de al menos esa clase entre las N primeras, para acceptable,
   medium y high. Añade también el techo rígido, el DockQ medio del top-1 y el tiempo medio
   de docking.

`BenchConfig::parallel_cases` reparte los casos entre núcleos. Conviene dejarlo apagado para
medir tiempos de un docker que ya es paralelo.

### Validación del arnés con dockers ficticios

`bench::fake` trae dos dockers de referencia:

- **Oráculo**: recupera la aleatorización superponiendo el ligando recibido sobre el
  original y devuelve la pose de referencia con ruido (rotación de hasta 5° y traslación de
  hasta 1 Å), seguida de poses aleatorias. Si la composición de transformaciones o la
  evaluación fallaran, no acertaría.
- **Aleatorio**: 100 poses con orientación uniforme y el ligando a una distancia de contacto
  aproximada (suma de radios de giro ±20 %).

Resultados sobre los 257 casos (`cargo run --release -p cs-core --example bench -- --docker
oracle|random`; unos 2 s por ejecución). Porcentaje de casos con éxito, en formato
acceptable / medium / high:

| Docker | Categoría | Top-1 | Top-10 | Top-100 | Techo rígido |
|---|---|---|---|---|---|
| oráculo | rigid (162) | 100 / 99 / 58 | 100 / 99 / 58 | 100 / 99 / 58 | 100 / 99 / 69 |
| oráculo | medium (60) | 100 / 95 / 7 | 100 / 95 / 7 | 100 / 95 / 7 | 100 / 95 / 7 |
| oráculo | difficult (35) | 94 / 54 / 0 | 94 / 54 / 0 | 94 / 54 / 0 | 94 / 51 / 0 |
| oráculo | total (257) | 99 / 92 / 38 | 99 / 92 / 38 | 99 / 92 / 38 | 99 / 91 / 45 |
| aleatorio | total (257) | 0 / 0 / 0 | 1 / 0 / 0 | 3 / 0 / 0 | 99 / 91 / 45 |

El oráculo acierta en el top-1 todos los casos cuyo techo es un acierto. El ruido de 5°/1 Å
lo baja de high a medium en algunos casos y, al revés, a veces lo mejora un poco, porque la
pose de referencia no minimiza exactamente cada métrica. El docker aleatorio no acierta nunca
en el top-1 y en el top-100 consigue un "acceptable" por azar en un 3 % de los casos.

El techo rígido muestra qué se puede esperar de un docking de cuerpos rígidos con
estructuras no unidas. "High" es casi inalcanzable en los casos medios y difíciles, y hay
casos que ni siquiera llegan a acceptable: 1H1V y 1FAK, por grandes cambios
conformacionales. También hay casos rígidos limitados: en 2O8V faltan en el receptor no unido
14 residuos de la interfaz y solo pueden reproducirse 22 de los 59 contactos nativos.

### Ejecutar el benchmark con un docker real

El docker de `cs_core::dock` encaja como cualquier función con la firma de arriba:

```rust
let report = bench::run("cshape-dock", &cases, dir, &config,
    |r, l, input| /* dock(r, l, …) → Vec<Rigid> ordenadas */, &progress)?;
```

El ejemplo `examples/bench.rs` muestra el uso completo, con filtros por caso (`--only`) y
categoría (`--category`) y salida en `report.json` y `report.md`.

## Limitaciones

- Las métricas son las de DockQ v2 con receptor y ligando como dos cuerpos. Las interfaces
  internas de un receptor de varias cadenas no se evalúan.
- Las asignaciones alternativas de cadenas solo cubren copias idénticas presentes en los
  archivos. Una pose que use una simetría que no está en el archivo, como la otra cara de un
  homodímero cuando solo se dio un monómero, cuenta como incorrecta.
- Solo se emparejan residuos idénticos en secuencia: un mutante puntual pierde ese residuo
  en fnat e iRMSD (igual que DockQ).
- La pose de referencia (`optimal_pose`) es una aproximación al techo rígido, no el óptimo
  exacto de cada métrica.
- La clase de CAPRI se calcula con la regla en cascada (mejor clase con fnat y una RMSD
  dentro de umbral). La tabla original de CAPRI tiene además casos raros con fnat alto y
  RMSD muy grandes que aquí cuentan como una clase inferior.
