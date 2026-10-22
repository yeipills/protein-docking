# Superficie excluida al solvente desde átomos

`cs_core::surface` calcula la superficie excluida al solvente (SES) directamente desde un
PDB, sin ejecutar MSMS. Devuelve una `Mesh` cerrada, orientada hacia afuera, con normales
unitarias por vértice, lista para el modo corregido de formas de contexto o para docking.

```rust
use cs_core::surface::{self, SesGrid, SesParams};

let s = cs_core::io::pdb::Structure::read(path)?;
let mesh = surface::ses(&s.atoms, &SesParams::default(), &NoProgress)?;

// O el campo, para consultas dentro/fuera rápidas (docking):
let grid = SesGrid::from_atoms(&s.atoms, &SesParams::default(), &NoProgress)?;
let d = grid.signed_distance(p); // < 0 dentro, > 0 fuera
let mesh = grid.mesh(&NoProgress)?;
```

Ejemplo de línea de comandos (escribe `.vert`/`.face` de MSMS e imprime estadísticas):

```bash
cargo run --release -p cs-core --example ses -- proteina.pdb salida/proteina \
    [--probe 1.4] [--spacing 0.5] [--radii bondi|united] [--reference msms/proteina]
```

## Método

Con radios expandidos `R_i = r_i + sonda`, el centro de la sonda puede estar en la región
`E` exterior a todas las esferas expandidas. Un punto está fuera de la SES si queda a menos
de `sonda` de `E`. El campo

```text
g(x) = sonda − dist(x, E)        (> 0 fuera, < 0 dentro, |∇g| = 1)
```

es una distancia con signo a la SES cerca de ella, y la SES es `g = 0`.

1. **Rejilla.** Caja de los átomos más `R_max + 3h` de margen (todo el borde es solvente),
   con espaciado `h`. Cada nodo guarda `dist(x, E)` cuantizada en `u16` sobre
   `[0, sonda + banda]` (resolución < 1e-4 Å, 2 bytes por nodo), con dos códigos
   reservados: fuera de toda esfera (`g = sonda`) y más profundo que la banda
   (`g = −banda`). La banda es `max(2,5 h, interior_band)`, lo que marching cubes necesita.
2. **Distancia exacta, sin transformada de distancia.** El borde de `E` está formado por
   casquetes de esfera expuestos, arcos donde se cortan dos esferas y vértices donde se
   cortan tres. El punto de `E` más cercano a un nodo interior está en uno de ellos, así
   que cada elemento se "pinta" sobre los nodos de su región y cada nodo se queda con el
   mínimo (mínimo atómico, en paralelo con rayon):
   - *Arcos.* Para cada par de esferas que se cortan se restan analíticamente los
     intervalos angulares de su circunferencia enterrados en vecinos comunes. Los nodos de
     la cuña entre las direcciones a ambos centros, dentro del rango angular del arco,
     reciben la distancia a la circunferencia.
   - *Vértices.* Los extremos de los arcos; cada nodo del cono formado por las direcciones
     a los tres centros recibe su distancia.
   - *Casquetes.* Un nodo dentro de la esfera `i` cuya proyección radial no queda enterrada
     en un vecino recibe `R_i − |x − a_i|`.

   A diferencia del enfoque de EDTSurf (vóxeles + transformada de distancia euclídea), no
   hay error de vóxel: los valores en los nodos son exactos.
3. **Cavidades** (`fill_cavities`, activo por defecto, como MSMS). Solo se pintan los
   elementos del borde de la componente exterior de `E`. Los arcos se agrupan en piezas
   conexas por sus vértices (como la *reduced surface* de MSMS; cada terna de esferas tiene
   dos vértices especulares y se distinguen) y por los casquetes que comparten (muestreo
   de cada esfera con 1024 direcciones). Una pieza es exterior si alguno de sus arcos ve en
   línea recta un nodo de solvente alcanzable desde el borde de la rejilla. Clasificar
   piezas enteras, y no nodos, conserva bolsillos estrechos sin nodos de rejilla y evita
   fusionar una cavidad con el exterior cuando sus sondas se solapan a través de una pared
   delgada. Los casquetes de una esfera que bordea a la vez el exterior y una cavidad toman
   la clase del arco más cercano (exacto: cualquier camino sobre la esfera hacia otro
   casquete cruza primero el borde del propio).
4. **Marching cubes estanco.** En lugar de la tabla de 256 casos (que deja grietas cuando
   dos cubos resuelven distinto una cara ambigua), cada cubo traza su contorno cara por
   cara: segmentos orientados con las esquinas positivas a la izquierda y las caras
   ambiguas resueltas con el *asymptotic decider* calculado de forma independiente del
   orden, así que los dos cubos vecinos coinciden. Los segmentos cierran en lazos
   disjuntos que se triangulan por orejas sin usar diagonales entre vértices de una misma
   cara del cubo (el cubo vecino podría usarlas también). Resultado: cada arista en
   exactamente dos triángulos, orientación consistente hacia `g > 0`, vértices soldados
   por arista de la rejilla.
5. **Vértices y normales.** El cruce en cada arista se toma de la cúbica que pasa por los
   cuatro nodos de esa línea (la interpolación lineal mete los vértices `≈ h²/8R` hacia el
   lado cóncavo). Las normales salen del gradiente del campo (diferencias centrales
   interpoladas).

### Parámetros (`SesParams`)

| campo | defecto | significado |
|---|---|---|
| `probe` | 1.4 | radio de la sonda (Å) |
| `grid_spacing` | 0.5 | espaciado de la rejilla (Å); los triángulos miden aproximadamente eso |
| `radii` | `bondi` | `bondi` (por elemento) o `united` (átomo unido por residuo y nombre: los radios con que se calcularon las superficies MSMS de los datos de ejemplo) |
| `include_hydrogens` | false | usar hidrógenos (con radios Bondi) |
| `include_hetatm` | true | usar HETATM (MSE, cofactores, iones; las aguas ya se descartan al leer) |
| `fill_cavities` | true | descartar las superficies de cavidades internas inaccesibles |
| `interior_band` | 0 | profundidad hasta la que `SesGrid::signed_distance` es exacta (mínimo `2,5 h`) |

`SesGrid::signed_distance` interpola trilinealmente; es exacta en los nodos dentro de
`[−banda, sonda]` y está saturada fuera de ese rango (fuera de la rejilla devuelve `sonda`).

## Validación

`crates/cs-core/tests/surface.rs`:

- **Un átomo** (`h = 0,2`): esfera de radio `r_vdw`; área y volumen dentro del 1 %, cada
  vértice a menos de 0,01 Å de la esfera y normales radiales.
- **Dos átomos solapados**: el surco reentrante está en `ρ − sonda` del eje (`ρ`, radio de
  la circunferencia de contacto de la sonda), según el campo y según la malla; el volumen
  coincide (1 %) con la integral exacta de la SES de dos esferas.
- **Cavidad**: una capa cerrada de átomos se rellena; con una boca amplia el interior queda
  como bolsillo abierto; sin `fill_cavities` la cavidad aparece como superficie interior.
- **Malla**: cerrada (cada arista en dos caras, en sentidos opuestos), volumen positivo,
  normales unitarias.
- **1AHW_l_u frente a MSMS** (sonda 1,5 Å, densidad 1 vértice/Å², mismos radios: la tabla
  `united` reproduce los 1622 radios que usó MSMS, recuperados de sus vértices):

  | `h` (Å) | vértices | área (Å²) | volumen (Å³) | MSMS→nuestra: media / p95 / máx (Å) | tiempo |
  |---|---|---|---|---|---|
  | 0,25 | 239 574 | 9984 | 27 548 | 0,004 / 0,009 / 0,27 | 0,48 s |
  | 0,35 | 121 948 | 9966 | 27 542 | 0,007 / 0,017 / 0,48 | 0,24 s |
  | **0,5** | 59 496 | 9932 | 27 530 | **0,014 / 0,034 / 0,46** | **0,14 s** |
  | 0,7 | 30 216 | 9871 | 27 506 | 0,026 / 0,066 / 0,62 | 0,08 s |
  | 1,0 | 14 666 | 9758 | 27 450 | 0,050 / 0,129 / 1,12 | 0,05 s |
  | MSMS | 11 141 | 9546 | 27 367 | | |

  Los vértices de MSMS están sobre la superficie exacta, así que su distancia a nuestra
  malla mide nuestro error, que baja como `h²`. En sentido contrario (nuestros vértices →
  triángulos de MSMS) la distancia es 0,106 / 0,242 / 0,63 Å para cualquier `h`: es la
  flecha de las cuerdas de MSMS, cuyos triángulos de ~1 Å cortan las zonas convexas y
  puentean las cóncavas; por lo mismo MSMS subestima el área un 4–5 % (nuestra área
  converge a ≈ 10 000 Å²). La distancia simétrica a `h = 0,5` es 0,091 Å de media y
  0,232 Å en el percentil 95.

  MSMS descarta dos cavidades diminutas de 1AHW_l_u (cerca de PHE 76 y LEU 108); con
  `fill_cavities` también desaparecen aquí. Sin la distinción de los dos vértices
  especulares de cada terna, una de ellas se fusionaba con el exterior (desvío de 2 Å).

**Espaciado por defecto: 0,5 Å.** El error frente a la superficie exacta (0,014 Å de media,
0,034 Å p95) es siete veces menor que el de la propia triangulación de MSMS, muy inferior
a la incertidumbre de las coordenadas cristalográficas y a las bandas de 0,2 Å de las capas
de contexto; la malla tiene ~6 vértices/Å² y 1AHW_l_u tarda 0,14 s. Bajar a 0,35 Å
duplica el tamaño de la malla y casi el tiempo para ganar centésimas de Å; subir a
0,7–1,0 Å sirve para cribado rápido.

Con radios Bondi (el defecto) la superficie de 1AHW_l_u difiere de la de MSMS en 0,26 Å de
media (volumen −6,9 %): no es error del método sino del juego de radios (Bondi da 1,70 Å al carbono; la
tabla de átomo unido, 1,74–2,0 Å con los hidrógenos incluidos). Para reproducir superficies
MSMS calculadas con esos radios, usar `radii: united`.

## Rendimiento

Release, 4 hilos, parámetros por defecto (sonda 1,4 Å, `h = 0,5`, radios Bondi):

| estructura | átomos pesados | rejilla | vértices | tiempo |
|---|---|---|---|---|
| 1AHW_l_u | 1622 | 102×160×130 | 62 k | 0,12 s |
| 1N2C_r_u | 15 771 | 218×248×221 | 324 k | 0,79 s |
| 4GAM_r_u | 17 307 | 241×147×273 | 359 k | 1,0 s |

(Medido con otros procesos compilando en la misma máquina; los tiempos son aproximados.)
Memoria: 2 bytes por nodo de rejilla más la malla (4GAM_r_u: 9,7 M nodos, 126 MB de pico
del ejemplo completo, escritura de archivos incluida). En 4GAM_r_u el tiempo se reparte
así: arcos expuestos 0,24 s, pintado de arcos y vértices 0,26 s, casquetes 0,08 s, marching
cubes 0,2 s, resto (interior, relleno desde el borde, clasificación) 0,2 s. Todo es
paralelo salvo el relleno desde el borde y la unión de piezas (milisegundos).

## Limitaciones

- La malla interpola linealmente entre vértices: las aristas vivas de la SES (cúspides
  donde la sonda se corta a sí misma) quedan redondeadas a escala `h`, y estructuras más
  delgadas que `h` pueden cambiar de topología.
- `signed_distance` solo es una distancia exacta en `[−banda, sonda]`; más afuera está
  saturada en `sonda` (es la distancia a `E`, no a la SES).
- La conectividad del solvente para las cavidades se decide con la rejilla (nodos
  alcanzables desde el borde, con línea de visión) y con el grafo de arcos; un bolsillo
  real cuya boca no resuelve la rejilla y que solo se une al exterior por casquetes muy
  estrechos (< 0,35 Å en el muestreo de la esfera) podría tratarse como cavidad.
- Los radios `united` cubren los 20 aminoácidos estándar (como las superficies de
  ejemplo); otros átomos usan Bondi. Con hidrógenos explícitos conviene usar `bondi`.
