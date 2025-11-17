# Referencia Rápida de Inconsistencias en Documentación

## 🔴 CRÍTICAS (Actuar Inmediatamente)

### 1. MIGRATION_GUIDE.md - Línea 89-90
**Problema**: Referencias a Backend/.../Script03.pyx (no existe)
**Solución**: Reemplazar con `backend/app/algorithms/context_rays.py[x]`
**Impacto**: Desarrolladores buscan código en directorio incorrecto
**Tiempo**: 5 minutos

### 2. MIGRATION_GUIDE.md - Línea 133-134
**Problema**: Referencias a Backend/.../Script04.pyx (obsoleto)
**Solución**: Reemplazar con `backend/app/algorithms/layer_evaluator.py[x]`
**Impacto**: Código obsoleto en lugar de implementación actual
**Tiempo**: 5 minutos

### 3. SETUP.md - Línea 157-159
**Problema**: Falta Socket Server en sección "Acceder a la Aplicación"
**Solución**: Agregar `- **Socket Server**: http://localhost:8080`
**Impacto**: Socket Server no mencionado en instrucciones principales
**Tiempo**: 2 minutos

### 4. MIGRATION_GUIDE.md - Línea 379-384
**Problema**: Resumen completo de referencias incorrectas (6 líneas)
**Solución**: Actualizar con rutas correctas a backend/app/algorithms/
**Impacto**: Tabla de referencia completamente incorrecta
**Tiempo**: 10 minutos

### 5. ARCHITECTURE_ANALYSIS.md - Línea 150
**Problema**: Diagrama confunde Backend/ (obsoleto) con backend/ (actual)
**Solución**: Clarificar que Backend/ es deprecated, backend/ es actual
**Impacto**: Confunde estructura del proyecto
**Tiempo**: 5 minutos

---

## 🟡 IMPORTANTES (Esta Semana)

### 6. MIGRATION_GUIDE.md - Línea 37
**Problema**: `Backend/C-lculos-Previos-main/.../Script01...`
**Solución**: Reemplazar con `backend/app/algorithms/surface_reader.py`
**Tiempo**: 3 minutos

### 7. MIGRATION_GUIDE.md - Línea 61
**Problema**: `Backend/C-lculos-Previos-main/.../Script02...`
**Solución**: Reemplazar con `backend/app/algorithms/centroid_calculator.py`
**Tiempo**: 3 minutos

### 8. MIGRATION_GUIDE.md - Línea 184
**Problema**: `Backend/.../Script05_preparacion_capas_unity.py`
**Solución**: Reemplazar con `backend/app/algorithms/unity_exporter.py`
**Tiempo**: 2 minutos

### 9. DEPLOYMENT.md - Línea 140-152
**Problema**: Cython compilation documentado pero sin claridad
**Solución**: Aclarar que es automático en Docker, manual: `python setup.py build_ext --inplace`
**Tiempo**: 10 minutos

### 10. README.md - Línea 44
**Problema**: Multiprocessing mencionado sin documentación
**Solución**: Crear sección en DEPLOYMENT.md o agregar a PERFORMANCE_IMPROVEMENTS.md
**Tiempo**: 15 minutos

---

## 🟢 MENORES (Este Mes)

| Archivo | Línea | Problema | Solución | Tiempo |
|---------|-------|----------|----------|--------|
| frontend/README.md | 8 | React 18 | React 18.3 (actual 18.3.1) | 1 min |
| frontend/README.md | 9 | TypeScript 5 | TypeScript 5.6 (actual 5.6.2) | 1 min |
| frontend/README.md | 10 | Vite 5 | Vite 5.4 (actual 5.4.11) | 1 min |
| README.md | 6 | Node 20 (vago) | Node 20+ o Node 20.0+ | 2 min |
| OPTIMIZATION_SUMMARY.md | 20 | Frontend/.env | frontend/.env (con nota) | 2 min |
| ARCHITECTURE_ANALYSIS.md | 1241 | Backend/ referencia | Aclarar legacy vs actual | 3 min |
| SETUP.md | 99 | Socket.IO sin detalles | Agregar puerto y descripción | 2 min |
| MIGRATION_GUIDE.md | 413 | Backend/.../proteinas/ | /backend/tests/data/ | 2 min |

---

## 📋 CHECKLIST DE CORRECCIONES

### ALTA PRIORIDAD (2-3 horas total)
- [ ] Línea 37: MIGRATION_GUIDE.md - Backend/ → backend/app/algorithms/surface_reader.py
- [ ] Línea 61: MIGRATION_GUIDE.md - Backend/ → backend/app/algorithms/centroid_calculator.py
- [ ] Línea 89-90: MIGRATION_GUIDE.md - Backend/.../Script03.pyx → backend/app/algorithms/context_rays.pyx
- [ ] Línea 133-134: MIGRATION_GUIDE.md - Backend/.../Script04.pyx → backend/app/algorithms/layer_evaluator.pyx
- [ ] Línea 184: MIGRATION_GUIDE.md - Backend/ → backend/app/algorithms/unity_exporter.py
- [ ] Línea 379-384: MIGRATION_GUIDE.md - Actualizar tabla completa
- [ ] Línea 150: ARCHITECTURE_ANALYSIS.md - Aclarar Backend/ vs backend/
- [ ] Línea 157-159: SETUP.md - Agregar Socket Server URL

### MEDIA PRIORIDAD (1 semana)
- [ ] DEPLOYMENT.md - Ampliar sección Cython compilation
- [ ] README.md línea 44 - Documentar Multiprocessing
- [ ] SETUP.md línea 99 - Agregar detalles a Socket.IO Server
- [ ] Estandarizar nomenclatura Backend/Frontend → backend/frontend

### BAJA PRIORIDAD (1 mes)
- [ ] frontend/README.md líneas 8-10 - Actualizar versiones exactas
- [ ] README.md línea 6 - Aclarar Node.js versión
- [ ] OPTIMIZATION_SUMMARY.md línea 20 - Aclarar Frontend/ obsoleto
- [ ] MIGRATION_GUIDE.md línea 413 - Backend/.../proteinas/ → backend/tests/data/

---

## 🚀 ESTRATEGIA DE CORRECCIÓN RECOMENDADA

**Fase 1 (2 horas)** - Correcciones críticas:
1. Actualizar MIGRATION_GUIDE.md completamente (todas las referencias Backend/)
2. Agregar Socket Server URL a SETUP.md
3. Revisar ARCHITECTURE_ANALYSIS.md diagrama

**Fase 2 (1 hora)** - Documentación ampliada:
4. Expandir DEPLOYMENT.md con Cython details
5. Agregar sección Multiprocessing a README o DEPLOYMENT

**Fase 3 (30 minutos)** - Refinamientos menores:
6. Actualizar versiones en frontend/README.md
7. Aclarar dependencias en README.md

---

## 📞 VERIFICACIÓN POST-CORRECCIÓN

Después de hacer cambios, verificar:
- [ ] Todas las referencias `Backend/` reemplazadas con `backend/`
- [ ] `SETUP.md` incluye Socket Server en sección de acceso
- [ ] `MIGRATION_GUIDE.md` tiene rutas correctas a `backend/app/algorithms/`
- [ ] `DEPLOYMENT.md` documenta compilación Cython
- [ ] No hay referencias a `Frontend/.env` sin aclaración
- [ ] Versiones exactas especificadas en `frontend/README.md`
- [ ] Nomenclatura consistente: siempre minúscula para directorios

