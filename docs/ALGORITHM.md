# Algoritmo

## Entrada

MSMS describe la superficie excluida al solvente con dos archivos de texto, ambos con tres
líneas de cabecera:

- `.vert`: `x y z nx ny nz cara esfera tipo` por vértice (normales hacia afuera).
- `.face`: `v1 v2 v3 tipo esfera` por triángulo, con índices de vértice **desde 1**.
  Tipo de cara: 1 = contacto (esfera atómica), 2 = toroidal, 3 = reentrante (sonda).

## Modo corregido

1. **Malla.** Se leen vértices y caras (convirtiendo los índices a base 0) y se orientan las
   caras hacia afuera usando las normales de vértice de MSMS.
2. **Puntos de superficie.** Centroides de las caras cuyo tipo no está en
   `skip_face_types` (por defecto se omite el tipo 1, como en el original).
3. **Puntos críticos.** Adelgazamiento voraz en el orden del archivo: se conserva un punto y
   se descartan todos los que estén a menos de `min_spacing` Å (rejilla hash, O(n)).
4. **Rayos.** `rays` direcciones casi uniformes (retícula de Fibonacci) de largo `radius`.
   Cada rayo tiene `samples` muestras en los centros de sus celdas:
   `t = (k + ½) / samples`.
5. **Distancia con signo.** Para cada muestra se busca el triángulo más cercano con un BVH
   (SAH por bins) y se calcula la distancia exacta. El signo se toma de la pseudonormal
   ponderada por ángulo de la característica más cercana (cara, arista o vértice), según
   Bærentzen y Aanæs (2005). Así el resultado es robusto ante pequeños agujeros de la malla
   y basta una consulta por punto. Las muestras consecutivas de un rayo reutilizan el último
   triángulo como cota inicial.
6. **Capas.** Con `layer_edges = [e0 … e8]` y distancia `d`:

   | capa | condición |
   |---|---|
   | `in4`, `in3`, `in2`, `in1` | `e0 ≤ d < e1`, `e1 ≤ d < e2`, `e2 ≤ d < e3`, `e3 ≤ d < e4` |
   | `out1`, `out2`, `out3` | `e4 ≤ d < e5`, `e5 ≤ d < e6`, `e6 ≤ d < e7` |
   | `out4` | `e7 ≤ d ≤ e8` |
   | `ses` | `|d| ≤ ses_half_width` |
   | `vol` | `d < 0` |

   La distancia de cada muestra también se guarda (`viewer.bin`), por lo que las capas se
   pueden redefinir sin volver a calcular la geometría.

El cálculo por punto crítico es independiente y se reparte entre todos los núcleos (rayon).

### Validación

- Distancias: en 300 puntos alrededor de la superficie de `1AHW_l_u` coinciden con la
  distancia mínima a todos los triángulos calculada por fuerza bruta (error < 1e-9 Å).
- Signo: coincide con el número de vueltas (*winding number*) calculado por fuerza bruta
  en al menos el 99 % de esos puntos (MSMS genera algunos triángulos degenerados).
- Coherencia: cada máscara de capa sale de su distancia, `vol` ↔ `d < 0`, alrededor de la
  mitad de cada esfera queda dentro y los puntos críticos respetan el espaciado.

## Modo fiel al original

Es un port línea por línea de `Script02_calculo_centroides.py`,
`Script03_rayos_contexto.py`, `Script04_evaluacion_capas.py` (con sus funciones Cython de
`Script04.pyx`) y `Script05_preparacion_capas_unity.py`, más la parte de trimesh que usaba
Script03 (`RayMeshIntersector.intersects_any` sin Embree: fase amplia con R-tree, plano,
baricéntricas por Cramer y tolerancias `1e-5`, `1e-13`, `-1e-6`). Cada expresión de coma
flotante respeta el orden de evaluación de Python/NumPy, y los números se escriben con las
mismas reglas que `repr(float)`.

Defectos del original que se conservan a propósito:

1. **Índices desplazados.** Los índices de MSMS empiezan en 1 pero se usan como índices de
   lista desde 0, así que cada centroide se calcula con los vértices siguientes. (El backend
   FastAPI v2 sí restaba 1, por lo que tampoco reproducía los resultados de la tesis.)
2. **Muestreo de la esfera.** `phi` y `theta` recorren ambos `[0, 2π]` con 10 valores: con
   `phi = 0` y `phi = 2π` los diez rayos coinciden, y la cobertura es muy desigual.
3. **Dentro/fuera.** Para cada segmento se llama a `intersects_any(origen = punto k,
   dirección = punto k+1)`: se pasa la **posición absoluta** del siguiente punto como si fuera
   una dirección, y se pregunta si el rayo infinito toca la malla en algún lugar.
4. **Bandera perdida.** Al releer `cr_totales`, Script04 descarta la primera de las 16
   banderas, así que cada rayo tiene 15 celdas desalineadas una posición con sus puntos.
5. **Capas radiales desde el origen.** `suma_capa` desplaza el punto SES a lo largo del
   vector que va del **origen de coordenadas** al punto, no a lo largo de la normal ni del rayo.
6. **`out4` invertido.** La condición `if d_out4 < 1` sobre un booleano conserva el borde
   `out4` justo cuando cae **fuera** de la esfera.
7. **Vecinos limitados.** El filtrado de puntos críticos solo descarta los 100 vecinos más
   cercanos dentro de 10 Å; en zonas densas quedan puntos más próximos.
8. **Resumen desordenado.** Script05 numera los centros después de `np.unique`, que los
   ordena; los números del resumen no corresponden a los de los archivos de capas.

### Validación

`tools/legacy-oracle/run.sh` extrae los scripts originales del historial de git y los ejecuta
sobre `data/samples/1AHW_l_u`. Solo se cambian rutas de archivos y se reemplaza una búsqueda
cuadrática de Script04 por un diccionario equivalente (mismo orden y mismos valores). Los
SHA-256 de sus 24 archivos de salida están en `crates/cs-core/tests/legacy_oracle.sha256`, y
la prueba `legacy_matches_original_scripts_byte_for_byte` exige que `cshape` produzca
exactamente los mismos bytes.

El STL de la muestra se generó desde la propia malla MSMS. Los archivos Unity históricos que
había en el repositorio se calcularon con otro STL (exportado aparte y no conservado): sus
160 centros coinciden exactamente, pero alrededor del 30 % de las líneas de capas difiere por
la triangulación distinta.
