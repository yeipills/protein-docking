# Migration Guide - Scientific Algorithms

> **Status**: Infrastructure complete, scientific algorithms need full migration

## Overview

The platform architecture (v2.0) is complete with:
- ✅ Multi-user authentication
- ✅ Database models
- ✅ API endpoints
- ✅ Task queue
- ✅ WebSocket notifications
- ✅ Docker configuration

**Pending**: Full migration of scientific algorithms from old codebase

---

## 📋 Migration Checklist

### ✅ Completed
- [x] Project restructure
- [x] Backend infrastructure
- [x] Database models (User, Job, Protein)
- [x] API endpoints
- [x] Authentication system
- [x] Celery task queue
- [x] Socket server
- [x] Docker orchestration
- [x] Algorithm stubs created

### 🚧 In Progress - Scientific Algorithms

#### 1. Script01 - Surface Reader (Priority: HIGH)
**Location**: `backend/app/algorithms/surface_reader.py`

**Source**: `Backend/C-lculos-Previos-main/Centroides de triangulos/Programa_python/Script01_lectura_caras_vertices.py`

**Task**: Migrate MSMS file reading logic

**Current State**: Basic implementation exists, needs completion

**What to do**:
```python
def read_surface_files(vertices_file: str, faces_file: str):
    # 1. Read .vert file (skip 3 header lines)
    # 2. Parse using regex split by spaces: re.split(r' +', line)
    # 3. Return array with 11 columns per vertex
    # 4. Read .face file (skip 3 header lines)
    # 5. Return array with 6 columns per face
    # 6. Preserve ALL data, not just x,y,z coordinates
```

**Original Code Reference**: Lines 13-74 in Script01

---

#### 2. Script02 - Centroid Calculator (Priority: HIGH)
**Location**: `backend/app/algorithms/centroid_calculator.py`

**Source**: `Backend/C-lculos-Previos-main/.../Script02_calculo_centroides.py`

**Task**: Calculate triangle centroids from faces

**What to do**:
```python
def calculate_centroids(vertices, faces):
    # 1. Extract only x,y,z coordinates from vertices (columns 1,2,3)
    # 2. For each face, get the 3 vertex indices
    # 3. Skip faces where type_face == 1 (column 4 in faces)
    # 4. Calculate centroid: (v1 + v2 + v3) / 3
    # 5. Return list of centroids
    # 6. Optionally: Export to centroidesPCS.txt
```

**Original Code Reference**: Lines 7-61 in Script02

**Key Logic**:
- Only process faces where `type_face != 1` (line 30)
- Extract vertices by index (lines 32-39)
- Average the 3 vertices (lines 41-46)

---

#### 3. Script03 - Context Rays (Priority: CRITICAL + Cython)
**Location**: `backend/app/algorithms/context_rays.py`

**Source**:
- `Backend/.../Script03_rayos_contexto.py` (Python)
- `Backend/.../Script03.pyx` (Cython optimization - NOT FOUND, may need to be created)

**Task**: Calculate context rays using spherical sampling

**What to do**:

**Step 1**: Python implementation
```python
def calculate_context_rays(protein_name, vertices, faces, centroids, output_dir):
    # 1. Load STL file with trimesh
    # 2. Filter centroids (remove those within 10Å distance using cKDTree)
    # 3. For each filtered centroid:
    #    a. Generate spherical ray samples (phi x theta grid)
    #    b. Use radius=3, delta=10 (configurable)
    #    c. Calculate ray endpoints using spherical coordinates
    # 4. For each ray:
    #    a. Divide into n=17 segments using np.linspace
    #    b. Use mesh.ray.intersects_any to check hits
    #    c. Store segment hit results
    # 5. Export two files:
    #    - {protein_name}_CRtotales.txt
    #    - {protein_name}_rayos_contexto.txt
```

**Key Parameters**:
- `radius = 3` (sphere radius for CR)
- `delta = 10` (sampling grid size: 10x10 = 100 rays per centroid)
- `n = 17` (segments per ray)
- `maxdis = 10` (filter distance for centroids)

**Critical Performance Section** (Lines 175-224):
- Ray-mesh intersection is SLOW (~34.5 minutes mentioned in comments)
- Uses `mesh_SES.ray.intersects_any()` for each segment
- This is where Cython optimization will help most

**Original Code Reference**: Lines 14-245 in Script03

---

#### 4. Script04 - Layer Evaluator (Priority: CRITICAL + Cython)
**Location**: `backend/app/algorithms/layer_evaluator.py`

**Source**:
- `Backend/.../Script04_evaluacion_capas.py` (Python)
- `Backend/.../Script04.pyx` (Cython optimizations)

**Task**: Evaluate 9 context shape layers

**What to do**:

**Step 1**: Migrate Cython helper functions to `backend/app/algorithms/cython_utils.pyx`:
```cython
# From Script04.pyx (204 lines of Cython code)
cpdef distancia_pto_lista(pto1, listado_ptos)  # Line 10-33
cpdef calcular_modulo_pto(pto)                  # Line 42-57
cpdef pto_en_esfera(radii, centro, pto)         # Line 60-81
cpdef suma_capa(pto, dist)                      # Line 83-99
```

**Step 2**: Python implementation
```python
def evaluate_layers(cr_totals_file, context_rays_file, output_dir):
    # 1. Read CR files
    # 2. Parse SES points and ray data
    # 3. For each SES point:
    #    a. Calculate 9 layers: in1-4, ses, out1-4
    #    b. Distances: -1, -0.8, -0.4, -0.2, 0, +0.2, +0.4, +0.8, +1 Angstroms
    #    c. Check if each layer point falls within sphere (radius=3)
    # 4. Evaluate each ray segment against layers
    # 5. Generate 10 output files per protein
    # 6. Return layer data dictionary
```

**Layers to Calculate**:
- `in1`: SES - 0.2Å
- `in2`: SES - 0.4Å
- `in3`: SES - 0.8Å
- `in4`: SES - 1.0Å
- `ses`: Solvent Excluded Surface (0)
- `out1`: SES + 0.2Å
- `out2`: SES + 0.4Å
- `out3`: SES + 0.8Å
- `out4`: SES + 1.0Å
- `vol`: Volumetric data

**Original Code Reference**:
- Python: Lines 1-439 in Script04
- Cython: Lines 1-204 in Script04.pyx

---

#### 5. Script05 - Unity Exporter (Priority: MEDIUM)
**Location**: `backend/app/algorithms/unity_exporter.py`

**Source**: `Backend/.../Script05_preparacion_capas_unity.py`

**Task**: Format layer data for Unity visualization

**What to do**:
```python
def export_for_unity(protein_name, layer_data, output_dir):
    # 1. Read rayos_contexto file to get CR indices
    # 2. For each of 9 layers + volume:
    #    a. Read layer file
    #    b. Reshape data (cantidad_cs x cantidad_rayos x segmentos)
    #    c. Format: [cs_number, ray_index, array_values, inicio_fin]
    # 3. Write 10 Unity files:
    #    - {protein}_cs_in1_unity.txt
    #    - {protein}_cs_in2_unity.txt
    #    - ... (all 9 layers)
    #    - {protein}_cs_vol_unity.txt
    # 4. Also create: {protein}_resumen_cs_unity.txt
```

**Key Variables**:
- `cantidad_segmentos = 15`
- `cantidad_rayos = 100`

**Original Code Reference**: Lines 1-151 in Script05

---

## 🔧 Cython Configuration

### Setup Files Needed

#### 1. `backend/setup.py`
```python
from setuptools import setup, Extension
from Cython.Build import cythonize
import numpy

extensions = [
    Extension(
        "app.algorithms.cython_utils",
        ["app/algorithms/cython_utils.pyx"],
        include_dirs=[numpy.get_include()],
    )
]

setup(
    name="protein-docking-algorithms",
    ext_modules=cythonize(
        extensions,
        compiler_directives={
            'language_level': "3",
            'boundscheck': False,
            'wraparound': False,
            'cdivision': True
        }
    ),
)
```

#### 2. `backend/app/algorithms/cython_utils.pyx`
```cython
# Migrate all Cython functions from Script04.pyx here
cimport cython
from libc.math cimport sqrt
import numpy as np

@cython.boundscheck(False)
cpdef double distancia_pto_lista(list pto1, list listado_ptos):
    # ... implementation from Script04.pyx line 10

@cython.boundscheck(False)
cpdef double calcular_modulo_pto(list pto):
    # ... implementation from Script04.pyx line 42

@cython.boundscheck(False)
cpdef bint pto_en_esfera(double radii, list centro, list pto):
    # ... implementation from Script04.pyx line 60

@cython.boundscheck(False)
cpdef list suma_capa(list pto, double dist):
    # ... implementation from Script04.pyx line 83
```

#### 3. Update `backend/Dockerfile`
```dockerfile
# In builder stage, add:
COPY setup.py .
RUN python setup.py build_ext --inplace
```

---

## 🧪 Testing Strategy

### 1. Unit Tests
Create `backend/tests/test_algorithms.py`:
```python
def test_surface_reader():
    # Test with sample .vert and .face files
    vertices, faces = read_surface_files("test.vert", "test.face")
    assert len(vertices) > 0
    assert len(faces) > 0

def test_centroid_calculator():
    # Test centroid calculation
    vertices = np.array([[0,0,0], [1,0,0], [0,1,0]])
    faces = np.array([[0,1,2]])
    centroids = calculate_centroids(vertices, faces)
    assert len(centroids) == 1

def test_context_rays():
    # Test CR generation
    ...
```

### 2. Integration Tests
```python
def test_part_one_pipeline():
    # Test complete Part One workflow
    # Upload files → process → verify output

def test_part_two_pipeline():
    # Test complete Part Two workflow
    # Upload CR files → process → verify layers
```

---

## 📁 File Structure After Migration

```
backend/app/algorithms/
├── __init__.py
├── surface_reader.py         ✅ Needs completion
├── centroid_calculator.py    ✅ Needs completion
├── context_rays.py           🚧 Critical - needs full migration
├── layer_evaluator.py        🚧 Critical - needs full migration
├── unity_exporter.py         ✅ Needs completion
└── cython_utils.pyx          ❌ Needs creation + Cython functions
```

---

## 🚀 Migration Steps (Recommended Order)

### Week 1: Core Algorithms
1. ✅ Complete Script01 (surface_reader.py)
2. ✅ Complete Script02 (centroid_calculator.py)
3. ✅ Test with sample protein files

### Week 2: Context Rays
4. 🔧 Implement Script03 Python version
5. 🔧 Test CR generation (expect slow performance)
6. 🔧 Profile to identify bottlenecks

### Week 3: Cython Optimization
7. 🎯 Create cython_utils.pyx with Cython functions
8. 🎯 Configure setup.py and Dockerfile
9. 🎯 Compile and test Cython modules
10. 🎯 Optimize Script03 with Cython

### Week 4: Layer Evaluation
11. 🔧 Implement Script04 Python version
12. 🔧 Integrate Cython helper functions
13. 🔧 Test layer generation

### Week 5: Unity Export + Polish
14. ✅ Implement Script05 (unity_exporter.py)
15. ✅ End-to-end testing
16. ✅ Performance optimization
17. ✅ Documentation

---

## 📊 Performance Expectations

### Before Cython:
- Script03 (Context Rays): **~34.5 minutes** per protein
- Script04 (Layer Evaluation): **~60 minutes** per protein

### After Cython:
- Script03: Expected **~5-10 minutes** (6-7x speedup)
- Script04: Expected **~10-15 minutes** (4-6x speedup)

### Bottlenecks:
1. **Ray-mesh intersections** (Script03 line 211)
2. **Distance calculations** (Script04 - Cython functions)
3. **Layer point evaluation** (Script04 lines 250-389)

---

## 🔗 Useful References

### Original Files:
- `Backend/C-lculos-Previos-main/Centroides de triangulos/Programa_python/Script01_lectura_caras_vertices.py`
- `Backend/.../Script02_calculo_centroides.py`
- `Backend/.../Script03_rayos_contexto.py`
- `Backend/.../Script04_evaluacion_capas.py`
- `Backend/.../Script04.pyx` (Cython)
- `Backend/.../Script05_preparacion_capas_unity.py`

### Libraries Used:
- `numpy` - Array operations
- `trimesh` - 3D mesh processing
- `scipy.spatial.cKDTree` - Nearest neighbor search
- `Cython` - Performance optimization

### Cython Documentation:
- https://cython.readthedocs.io/
- https://cython.readthedocs.io/en/latest/src/userguide/numpy_tutorial.html

---

## ❓ FAQ

**Q: Can I test without migrating algorithms?**
A: Yes! The infrastructure is complete. You can test authentication, job creation, WebSocket notifications, etc. Algorithms only run during actual protein processing.

**Q: Do I need to migrate everything at once?**
A: No. You can migrate incrementally:
1. Start with Script01 & Script02 (simple)
2. Then tackle Script03 (complex but critical)
3. Finally Script04 with Cython optimization

**Q: What if I don't know Cython?**
A: Start with Python implementations. They will work but be slow. Learn Cython later for optimization. The platform will function with pure Python.

**Q: How do I test algorithms?**
A: Use sample protein files from `Backend/.../proteinas/` directory. Start with small proteins for faster iteration.

**Q: Can I get help?**
A: Yes! Each script has detailed comments in the original source files. You can also refer to the inline documentation and code examples.

---

## 📝 Next Steps

1. **Read this guide thoroughly**
2. **Start with Script01** (easiest)
3. **Test each script** with sample data
4. **Move to Script03** (most critical)
5. **Add Cython** when Python version works
6. **Integrate with Celery tasks** when complete

Remember: **The infrastructure is done. Focus only on the scientific algorithms.**

Good luck! 🚀
