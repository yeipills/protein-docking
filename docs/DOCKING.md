# Docking rígido por formas de contexto complementarias

El módulo `cs_core::dock` acopla dos proteínas rígidas (receptor fijo, ligando móvil)
buscando parches de superficie con formas complementarias, a la manera de Shentu, Al Hasan,
Bystroff y Zaki, *Context shapes: Efficient complementary shape matching for protein–protein
docking* (Proteins 2008). Después vuelve a puntuar las mejores soluciones con los átomos,
las refina localmente y las agrupa.

Es una línea base clásica de complementariedad de forma: correcta, rápida y probada, pero no
compite con los métodos actuales (ver [Resultados](#resultados) y [Limitaciones](#limitaciones)).

## Uso

```rust
use cs_core::dock::{self, AtomSurfaceParams, DockInput, DockParams};
use cs_core::progress::NoProgress;

let receptor = DockInput::from_atoms(receptor_pdb, &AtomSurfaceParams::default(), &NoProgress)?;
let ligand = DockInput::from_atoms(ligand_pdb, &AtomSurfaceParams::default(), &NoProgress)?;
// o bien DockInput::from_ses(estructura, &SesParams::default(), …) (generador SES)
// o DockInput::from_mesh(estructura, malla) (MSMS u otra malla cerrada)
let poses = dock::run(&receptor, &ligand, &DockParams::default(), &NoProgress)?;
for p in &poses[..10] {
    // p.transform: Rigid que se aplica a las coordenadas de entrada del ligando
    println!("{} {:.1} {:?}", p.rank, p.score, p.components);
}
let pdb = dock::complex_pdb(&receptor.structure, &ligand.structure, &poses[0].transform);
```

`run_with_stats` devuelve además conteos y tiempos por etapa. El cálculo es determinista (no
hay azar y los resultados paralelos se combinan en orden fijo) y se puede cancelar con
`Progress::is_cancelled`.

Desde la CLI, el servidor y la web (ver el [README](../README.md)):

```bash
cshape dock --receptor r_u.pdb --ligand l_u.pdb \
    [--native-receptor r_b.pdb --native-ligand l_b.pdb] [--surface atoms|ses] \
    [--randomize-seed N] [--params docking.json] --out dock/
cshape bench --category rigid --out bench/
```

En la web, la pestaña *Docking* acepta receptor y ligando (y opcionalmente el complejo
unido), y muestra las poses con su calidad CAPRI/DockQ y un visor del complejo.

Ejemplo de línea de órdenes del crate (escribe `poses.json` y los complejos en PDB):

```bash
cargo run --release -p cs-core --example dock -- \
    data/complexes/1AHW/1AHW_r_u.pdb data/complexes/1AHW/1AHW_l_u.pdb \
    --randomize 7 --native data/complexes/1AHW/1AHW_r_b.pdb data/complexes/1AHW/1AHW_l_b.pdb
```

`--surface ses` usa la superficie del generador SES (`SesSurface`) en vez del modelo de átomos, `--params` acepta
cambios de `DockParams` en JSON (`'{"n_poses":50}'` o `@archivo.json`) y `--native` informa
la clase CAPRI y el DockQ de cada pose con `cs_core::eval`.

## Representación de la superficie

El motor no depende de cómo se describe la superficie. El rasgo `Surface` pide solo dos cosas:

- `signed_distance(p)`: distancia con signo (negativa dentro), exacta cerca de la superficie;
- `surface_points()`: puntos densos sobre la superficie con su normal hacia afuera.

Hay tres implementaciones:

| Tipo | Distancia | Puntos |
|---|---|---|
| `MeshSurface` (malla cerrada, p. ej. MSMS) | exacta a los triángulos con BVH (`geom::sdf`) | vértices y normales de la malla |
| `SesSurface` (generador SES, `DockInput::from_ses`) | campo exacto de `SesGrid` entre −4 Å y la sonda; fuera de ese rango, distancia a su malla | vértices de la malla |
| `AtomSurface` (solo átomos, `DockInput::from_atoms`) | rejilla de la SES por transformadas de distancia (ver abajo) | nodos de la banda \|d\| < h/2 proyectados sobre el cero |

`AtomSurface` construye la SES de los átomos pesados (radios de Bondi, sonda 1,4 Å) en una
rejilla de 0,5 Å: marca los nodos dentro de la SAS, calcula con la transformada de distancia
euclídea exacta (Felzenszwalb y Huttenlocher) la distancia `E` a la región accesible para el
centro de la sonda; los puntos con `E > sonda` forman el interior de la SES, donde la
distancia es exactamente `sonda − E`; fuera, es la transformada de distancia hasta ese
interior. Las cavidades internas se rellenan (como MSMS). Error típico ±0,25 Å; 0,1–0,3 s
por proteína.

El generador SES del repositorio (`cs_core::surface`) entra como `SesSurface`. Su `SesGrid`
solo no basta: limita la distancia exterior al radio de sonda (1,4 Å) y la interior a una
banda, y los descriptores necesitan distinguir hasta 3 Å fuera y casi 4 Å dentro. La malla
sola tampoco: el signo por pseudonormales falla en un 2–3 % de los puntos profundos (triángulos
casi degenerados de *marching cubes*). `SesSurface` usa el campo exacto de la rejilla (con la
banda interior subida a 4 Å) y recurre a la malla solo fuera de ese rango, con el signo de la
rejilla. `SesSurface` y `AtomSurface` coinciden: diferencia media de 0,15–0,3 Å entre −4,5 y
4,5 Å de la superficie de 1AHW_r.

## Algoritmo

### 1. Puntos críticos y marcos locales

Los puntos de superficie se adelgazan de forma voraz para que ningún par quede a menos de
`spacing` Å (2,5 Å). Cada punto crítico recibe una **normal de parche**: la media, ponderada
linealmente por la distancia, de las normales de los puntos de superficie a menos de
`normal_radius` (5 Å). La normal local de una SES cambia mucho de un átomo a otro; para un
contacto interesa la orientación del parche. El marco local es `[x, y, n]` con `x`
arbitrario pero determinista, porque se prueban todas las rotaciones alrededor de `n`.

Los puntos del ligando reciben además **marcos inclinados**: el marco base y seis copias
inclinadas 12° en seis direcciones (`tilt_angles`, `tilt_azimuths`). Las normales de parche
de dos superficies en contacto difieren 10–20°, y ese error, multiplicado por el radio del
ligando (20–30 Å), dominaba el error de las poses: con un solo marco por punto, aun eligiendo
el mejor par cercano y la mejor rotación, la pose típica quedaba a ~10 Å del nativo; con las
inclinaciones, a 4–7 Å.

### 2. Formas de contexto con rotación por desplazamiento de bits

El entorno de cada punto se muestrea en una rejilla cilíndrica del marco local:
`ring_radii` = 1,5…9 Å (6 radios) × `heights` = −3,75…3,75 Å (6 alturas) = 36 anillos de
`K = 32` muestras, la muestra `a` a un ángulo `2πa/K`. Una rotación de `2π/K` (11,25°)
alrededor de la normal es un desplazamiento cíclico de cada anillo: un `rotate_left` de un
`u32`. Cada muestra se clasifica por su distancia con signo `d` en cuatro máscaras de bits:

| Máscara | Condición | Sentido |
|---|---|---|
| `inside` | d < 0 | materia |
| `core` | d < −1,5 Å | materia profunda: solaparla es un choque |
| `shell` | 0 ≤ d < 3 Å | capa exterior que la pareja debería llenar |
| `surf` | \|d\| < 1 Å | capa de superficie (la capa SES del artículo) |

El ligando se muestrea ya en la geometría acoplada: volteado (normal opuesta) y en espejo, de
modo que su anillo `r`, bit `a`, cae sobre el anillo `r`, bit `a + k` del receptor en la
pose de rotación `k`. Una prueba unitaria verifica esta correspondencia muestra a muestra.

### 3. Emparejamiento: AND + popcount

Para cada punto del receptor `p`, cada marco del ligando `q` y cada rotación `k`, con
`L = rotl(L, k)`:

```
relleno  = Σ popcount((R.shell & L.inside) | (R.inside & L.shell))
contacto = Σ popcount(R.surf & L.surf)
choque   = Σ popcount((R.core & L.inside) | (R.inside & L.core))
puntuación = 1·relleno + 1·contacto − 3·choque
```

Las 32 rotaciones de un par se calculan juntas para que el compilador vectorice sobre ellas
(un carril SIMD por rotación). El núcleo se compila tres veces (genérico, AVX2 y AVX-512) y se
elige en tiempo de ejecución; las tres versiones se comparan con una referencia bit a bit.
Con 4 núcleos se evalúan ~5 millones de pares por segundo (×32 rotaciones = 160 millones de
configuraciones por segundo, 36 anillos × 3 popcount cada una).

Por cada punto del receptor se guardan los 30 mejores marcos del ligando (cada uno con su
mejor rotación) y, de todos ellos, los 60 000 mejores. Repartir las semillas por puntos del
receptor evita que un solo bolsillo se lleve todas.

### 4. Poses

Una semilla `(p, q, k)` define la transformación rígida del ligando que lleva `q` a `p`
(más `gap` a lo largo de la normal, 0 por defecto), con la normal de `q` opuesta a la de `p`
y girada `2πk/K`:

```
R = A_p · Rz(2πk/K) · diag(1, −1, −1) · A_qᵀ,   t = p + gap·n_p − R·q
```

(`A` = matriz con columnas `x, y, n`). Las semillas que colocan el ligando de forma casi
igual que otra mejor (misma celda de 2 Å para el centroide y dos átomos lejanos) se descartan.

### 5. Puntuación con átomos, refinamiento y agrupamiento

El descriptor solo ve un parche de ~10 Å; la interfaz completa se evalúa con los átomos
pesados. Con `d` = distancia del centro de un átomo a la superficie de van der Waals de la
pareja (negativa dentro de su volumen excluido al solvente, con la profundidad real):

| Término | Definición |
|---|---|
| contactos | átomos con 1,0 ≤ d ≤ 2,8 Å (pares a ~3–4,5 Å), rampas de 0,5 y 1 Å; media de los dos lados |
| choque | Σ max(0, 1,0 − d)² en los dos lados, promediado |
| potencial | potencial de contacto por clases de átomo (apolar, polar, catión, anión) hasta 6 Å |
| propensión | propensión de interfaz del residuo (Trp, Tyr, Met, Phe… positiva; Asp, Glu, Lys… negativa), ponderada por contacto |

`total = contactos − 3·choque − potencial + 3·propensión`.

Todo son consultas a rejillas precalculadas (0,75 Å): campos de distancia de cada proteína
y del potencial del receptor. Dos detalles importan:

- **Interior macizo.** La unión de esferas de van der Waals es porosa: entre átomos
  enterrados quedan huecos que contarían como "fuera", y un ligando que atraviesa el receptor
  acumularía contactos en vez de choques. El interior se define como esferas más todo hueco
  inaccesible a una sonda de 1,4 Å, y su profundidad se mide con una transformada de distancia.
- **Descarte por grupos.** Los átomos se agrupan de 8 en 8 (consecutivos, radio ≤ 6 Å) y
  una rejilla gruesa de cotas inferiores descarta en una consulta los grupos lejos de la
  pareja. Es exacto (una prueba lo compara con la evaluación completa) y deja cada pose en
  ~50–100 µs.

Las semillas (en orden de puntuación del descriptor) siguen dos refinamientos rígidos por
búsqueda de patrones (traslaciones ±paso en los ejes y rotaciones alrededor del centro del
ligando que mueven sus átomos más lejanos 1,5 pasos; se acepta la primera mejora y el paso se
divide a la mitad al atascarse):

1. las 2000 mejores, 40 evaluaciones con una puntuación blanda (contacto hasta 3,3 Å,
   choque desde 0 Å, peso 1), porque una semilla solo es precisa a unos Å;
2. las 1000 mejores según la puntuación final, 150 evaluaciones con ella. Hacen falta
   tantas porque, antes del refinamiento fino, una pose casi nativa suele quedar entre los
   puestos 150 y 1000: la puntuación de átomos solo la reconoce cuando ya está bien colocada.

Por último se agrupan de forma voraz por RMSD de Cα del ligando (4 Å) y se devuelven los
representantes (100 por defecto) con el tamaño de su grupo.

## Parámetros principales (`DockParams`)

| Parámetro | Defecto | |
|---|---|---|
| `descriptor.spacing` | 2,5 Å | separación de puntos críticos |
| `descriptor.normal_radius` | 5 Å | radio de la normal de parche |
| `descriptor.ring_radii` / `heights` | 1,5…9 / ±0,75…3,75 Å | geometría de los anillos |
| `descriptor.shell` / `core` / `surf` | 3 / 1,5 / 1 Å | umbrales de las máscaras |
| `descriptor.tilt_angles` / `tilt_azimuths` | [12°] / 6 | marcos inclinados del ligando |
| `weights` | relleno 1, contacto 1, choque 3 | pesos del emparejamiento |
| `per_point` / `max_candidates` | 30 / 60 000 | semillas por punto y en total |
| `quick_top` / `refine_top` | 2000 / 1000 | poses refinadas en cada etapa |
| `score` | ver arriba | puntuación final |
| `cluster_radius` / `n_poses` | 4 Å / 100 | agrupamiento y salida |

## Resultados

Todas las cifras son de esta máquina (4 núcleos Intel Xeon a 2,8 GHz, compilación release) y
se evalúan con `cs_core::eval` (clases CAPRI y DockQ, validados contra DockQ oficial; ver
[EVALUATION.md](EVALUATION.md)). El arnés aleatoriza la orientación y la posición del ligando
no unido con una semilla por caso, acopla y evalúa las 100 primeras poses.

### Categoría rígida completa del BM5.5 (162 casos, ligando no unido)

Porcentaje de casos con alguna pose de al menos esa clase entre las N primeras
(acceptable / medium / high):

| Superficie | Top-1 | Top-5 | Top-10 | Top-100 | DockQ medio top-1 | Docking medio (mediana, máx.) |
|---|---|---|---|---|---|---|
| `AtomSurface` (por defecto) | 4 / 2 / 0 | 15 / 7 / 1 | 20 / 8 / 1 | 43 / 15 / 1 | 0,056 | 9,3 s (7,9 s; 42 s) |
| `SesSurface` (generador SES) | 4 / 2 / 0 | 12 / 6 / 1 | 16 / 6 / 1 | 36 / 12 / 1 | 0,051 | 13,1 s (11,0 s; 64 s) |
| techo rígido (`optimal_pose`) | 100 / 99 / 69 | | | | | |
| docker aleatorio (EVALUATION.md, 257 casos) | 0 / 0 / 0 | | 1 / 0 / 0 | 3 / 0 / 0 | | |

Por tipo de complejo (casos con una pose aceptable en el top-N, `AtomSurface`):

| Tipo | Casos | Top-1 | Top-10 | Top-100 | Top-10 medium |
|---|---|---|---|---|---|
| anticuerpo–antígeno (AA, AS) | 44 | 1 | 7 | 18 | 2 |
| enzima–inhibidor (EI) | 38 | 5 | 15 | 25 | 8 |
| enzima–sustrato / regulador (ES, ER) | 23 | 0 | 3 | 7 | 0 |
| otros (OG, OR, OX) | 57 | 0 | 7 | 20 | 3 |
| total | 162 | 6 | 32 | 70 | 13 |

Los parámetros se ajustaron a mano mirando 12 casos de esta misma categoría (1AHW, 1MLC,
1AVX, 1BVN, 1PPE, 2SNI, 2PCC, 1AK4, 7CEI, 1E6E, 1Z0K, 2OUL). Sin ellos, el éxito en los otros
150 casos es 18 % en top-10 y 41 % en top-100: el sesgo del ajuste es pequeño, pero existe.
El ajuste se hizo con `AtomSurface`; `SesSurface` usa los mismos valores, lo que explica en
parte que rinda algo menos. Las dos superficies aciertan casi los mismos casos en el top-100
(49 ambos, 21 solo átomos, 10 solo SES): buena parte de la diferencia es el ruido descrito en
[Limitaciones](#limitaciones), no una superficie mejor que otra.

El techo rígido se alcanza en todos los casos (toda la categoría tiene una colocación rígida
aceptable del ligando no unido) y "high" en el 69 %. La distancia entre ese techo y el 43 %
del top-100 es la limitación del método, no de las estructuras.

### 1AHW (Fab 5G9 – factor tisular)

| Experimento | Resultado |
|---|---|
| Arnés (no unido, semilla 1), `AtomSurface` | primera pose aceptable y medium en el puesto 20 (DockQ 0,66) |
| Arnés (no unido, semilla 1), `SesSurface` | ninguna aceptable en el top-100 (mejor DockQ 0,10) |
| No unido, 6 orientaciones de partida (`--randomize 1…6`) | aceptable (medium) en los puestos 21, 11 y 20 en tres de ellas; nada en el top-100 en las otras tres |
| Redocking unido, 6 orientaciones de partida | medium en los puestos 1, 3, 1, 1, 3 en cinco de ellas (DockQ hasta 0,9); nada en una |
| Pose nativa refinada (no unido, semilla 2) | puntuación 125,8 frente a 154,4 de la mejor pose encontrada |

Tiempos de 1AHW no unido (`examples/dock.rs --randomize 2`):

| Etapa | `AtomSurface` | `SesSurface` |
|---|---|---|
| superficies (receptor y ligando en paralelo) | 0,50 s | 0,90 s |
| puntos críticos y descriptores (2074 del receptor, 8428 marcos del ligando) | 0,22 s | 4,36 s |
| emparejamiento (17,5 M pares × 32 rotaciones; 5,6 M pares/s) | 3,1 s | 3,3 s |
| poses, rejillas de átomos y dos refinamientos | 7,4 s | 8,2 s |
| **total** | **11,3 s** | **16,7 s** |

Con `SesSurface` los descriptores son más lentos porque las muestras más alejadas de la
superficie (fuera del rango de la rejilla) consultan la malla con el BVH.

## Limitaciones

- **La puntuación es el cuello de botella.** En muchos fallos hay semillas casi nativas
  entre las primeras miles, pero la puntuación de átomos (contactos, choques, potencial y
  propensión, todo a mano) no las pone arriba: prefiere interfaces grandes y bien empaquetadas
  aunque no sean las nativas. En el 1AHW no unido la pose nativa refinada puntúa 126 y la
  mejor encontrada 154. Un potencial estadístico ajustado (p. ej. por pares de residuos o
  tipos de átomo, entrenado fuera del benchmark), electrostática y desolvatación reales, o
  un re-ranking aprendido son el siguiente paso natural.
- **El descriptor local discrimina poco.** Un parche de ~10 Å encaja bien en muchos sitios:
  el par nativo suele estar en el puesto 100–1000 de su punto del receptor. Por eso hacen
  falta 60 000 semillas y refinar 2000. Votación de varias semillas consistentes (agrupar
  en el espacio de poses antes de puntuar) o descriptores de dos puntos ayudarían.
- **Sensibilidad a la orientación de partida.** Los puntos críticos y los marcos se mueven
  con el ligando (rejilla alineada con los ejes, adelgazamiento voraz en orden de entrada), y
  como el par nativo no destaca, pequeñas diferencias cambian si entra o no entre las
  semillas refinadas: el mismo 1AHW acierta desde unas orientaciones y no desde otras. Las
  cifras del benchmark son de una sola orientación por caso; con varias semillas cambiarían
  algunos puntos arriba o abajo.
- **Cuerpos rígidos.** Los choques se toleran hasta ~1 Å por átomo, pero no hay flexibilidad
  de cadenas laterales ni de bucles; los casos de dificultad media y difícil no se evaluaron.
- **Resolución.** 11,25° alrededor de la normal, inclinaciones de 12° y puntos cada 2,5 Å;
  el refinamiento local recupera el resto solo si la semilla cae en la cuenca correcta.
- **Coste.** La memoria crece con el tamaño del receptor (rejillas de 0,5 y 0,75 Å) y el
  emparejamiento con el producto de puntos críticos: 10–40 s para los complejos más grandes
  de la categoría rígida (hemaglutinina, 4GXU).

### Siguientes pasos

1. Sustituir el potencial y la propensión hechos a mano por un potencial estadístico
   entrenado en complejos fuera del BM5.5 y añadir electrostática.
2. Votación/agrupamiento de semillas antes del refinamiento para premiar las regiones que
   muchas semillas independientes señalan.
3. Promediar sobre varias orientaciones de partida o hacer los marcos independientes de la
   orientación (p. ej. puntos críticos por muestreo determinista sobre la superficie).
4. Afinar los parámetros para `SesSurface` y acelerar su rango lejano (extender la banda
   exterior de la rejilla en vez de consultar la malla).

## Pruebas

- Unitarias (`cargo test -p cs-core --lib dock`): transformada de distancia frente a fuerza
  bruta, interpolación, esfera y cuello de la SES de átomos, cavidades rellenas, marcos
  ortonormales, correspondencia exacta de muestras bajo la pose de cada rotación, núcleos
  genérico/AVX2/AVX-512 frente a la definición bit a bit, enumeración, términos de contacto y
  choque, profundidad de solapamiento, exactitud del descarte por grupos, refinamiento
  monótono y agrupamiento.
- Integración (`tests/dock.rs`): un cuerpo con un bolsillo y otro con la protuberancia
  complementaria se acoplan correctamente, el resultado es determinista, la cancelación
  funciona, el PDB del complejo contiene ambas partes, y el modelo de átomos, la malla MSMS,
  la malla SES y `SesSurface` coinciden dentro/fuera en `1AHW_l_u` (≈9 s en debug). Las
  pruebas con proteínas reales son lentas sin optimizar y están marcadas `#[ignore]`
  (`cargo test --release -p cs-core --test dock -- --ignored`, ~2 min). Cada una acopla
  1AHW desde tres orientaciones de partida, porque el resultado depende de ella: redocking
  unido con `AtomSurface` (medium en el top-10 en al menos dos), redocking unido con
  `SesSurface` y no unido con `AtomSurface` (aceptable en el top-100 en al menos una).
- Banco de pruebas: `examples/dock_bench.rs` ejecuta el arnés de `cs_core::eval::bench`
  (`--category rigid`, `--only …`, `--ses`) o, con `--diagnose`, informa en qué etapa
  aparece la primera pose aceptable.
