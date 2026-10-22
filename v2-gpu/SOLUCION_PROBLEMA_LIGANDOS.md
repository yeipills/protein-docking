# 🧪 SOLUCIÓN: Problema de Selección de Ligandos

## 🚨 **PROBLEMA IDENTIFICADO**

**Síntoma**: No puedes seleccionar moléculas de ligando en el formulario de docking.

## 🔍 **DIAGNÓSTICO IMPLEMENTADO**

He agregado un **sistema de diagnóstico completo** al DockingJobCreator que te mostrará exactamente qué está pasando:

### 📊 **Panel de Diagnóstico Visual**
Ahora verás:
- **Total Files**: Cuántos archivos tienes subidos
- **Receptor Files (>100 atoms)**: Cuántos califican como receptores
- **Ligand Files (≤100 atoms)**: Cuántos califican como ligandos
- **File Breakdown**: Lista detallada de cada archivo con su clasificación

### 🎛️ **Controles de Debugging**
- **🔍 Test**: Prueba la conexión con el backend
- **🔄 Refresh**: Recarga y reclasifica todos los archivos

## 🔧 **POSIBLES CAUSAS Y SOLUCIONES**

### 1. **No Tienes Archivos de Ligando** ❌
**Causa**: Todos tus archivos tienen >100 átomos (son receptores)

**Solución**:
```bash
# Descargar ligandos de ejemplo desde PubChem:
# Aspirina (21 átomos): https://pubchem.ncbi.nlm.nih.gov/compound/2244
# Cafeína (24 átomos): https://pubchem.ncbi.nlm.nih.gov/compound/2519
# Glucosa (24 átomos): https://pubchem.ncbi.nlm.nih.gov/compound/5793
```

**Características de ligandos válidos**:
- ✅ 10-100 átomos
- ✅ Formatos: .mol, .sdf, .mol2, .pdb
- ✅ Moléculas pequeñas (fármacos, metabolitos, etc.)

### 2. **Archivos No Están Siendo Parseados** ❌
**Causa**: El backend no puede analizar la estructura molecular

**Solución**:
1. Haz clic en **🔄 Refresh** para recargar archivos
2. Verifica en **File Breakdown** si aparece "No info" en átomos
3. Si sigue sin info, el archivo puede estar corrupto

### 3. **Backend Desconectado** ❌
**Causa**: No hay comunicación con el servidor Rust

**Solución**:
1. Verifica que el **🌐 Backend** esté en ✅ verde
2. Haz clic en **🔍 Test** para probar conexión
3. Si falla, reinicia el servidor backend

### 4. **Error en la Clasificación** ❌
**Causa**: Bug en el algoritmo de clasificación

**Solución**:
```typescript
// El sistema clasifica así:
receptorFiles = files.filter(f => f.protein_info?.total_atoms > 100);
ligandFiles = files.filter(f => f.protein_info?.total_atoms <= 100);
```

## 📋 **PASOS DE DIAGNÓSTICO**

### Paso 1: Verificar Estado Visual
1. Ve al tab **"Create Docking Job"**
2. Observa el **Debug Status Bar**:
   - ✅ **🧪 Ligand**: Debe estar verde si tienes ligandos
   - ✅ **🌐 Backend**: Debe estar verde
3. Revisa el **📊 File Analysis**

### Paso 2: Analizar Archivos
1. Mira **"Ligand Files (≤100 atoms)"**: debe ser > 0
2. Revisa **File Breakdown** para ver cada archivo:
   ```
   ejemplo.mol    25 atoms    🧪 Ligand
   proteina.pdb   1500 atoms  🧬 Receptor
   ```

### Paso 3: Acciones Correctivas
```bash
# Si no hay ligandos:
1. Haz clic en "🔄 Refresh"
2. Ve al tab "Upload Files"  
3. Sube moléculas pequeñas (10-100 átomos)

# Si hay problemas de conexión:
1. Haz clic en "🔍 Test"
2. Verifica que el backend esté corriendo
3. Revisar console del navegador (F12)
```

## 🧪 **Ejemplos de Ligandos Válidos**

### **Moléculas Pequeñas Comunes**:
```
Aspirina:     C9H8O4      (21 átomos)  ✅
Cafeína:      C8H10N4O2   (24 átomos)  ✅
Ibuprofeno:   C13H18O2    (33 átomos)  ✅
Glucosa:      C6H12O6     (24 átomos)  ✅
ATP:          C10H16N5O13P3 (47 átomos) ✅
```

### **NO Son Ligandos**:
```
Proteína completa:     >1000 átomos  ❌
Dominio proteico:      200-500 átomos ❌
Péptido largo:         >100 átomos   ❌
```

## 🔧 **Si Persiste el Problema**

### **Logging Detallado** (F12 → Console):
```javascript
// Busca estos logs:
"📊 DockingJobCreator: File classification results:"
"🔄 DockingJobCreator: Refreshing files and reclassifying..."
"📁 DockingJobCreator: Selecting ligand file:"
```

### **Verificaciones Manuales**:
1. **Backend corriendo**: http://localhost:8080/health
2. **Archivos en servidor**: http://localhost:8080/files
3. **Estado de parsing**: Revisa logs del servidor Rust

### **Reset Completo**:
```bash
# Si nada funciona:
1. Ve al tab "Upload Files"
2. Sube NUEVOS archivos de ligando
3. Haz clic en "🔄 Refresh" en Create Job
4. Verifica File Breakdown
```

## ✅ **Estado Correcto**

Cuando todo funcione correctamente verás:
```
Debug Status Bar:
📄 Receptor: ✓  🧪 Ligand: ✓  ⚙️ Config: ✓  🎯 Ready: ✓  🌐 Backend: ✓

File Analysis:
Total Files: 3
Receptor Files: 1  
Ligand Files: 2

File Breakdown:
proteina.pdb     1500 atoms    🧬 Receptor
aspirina.mol     21 atoms      🧪 Ligand  
cafeina.sdf      24 atoms      🧪 Ligand
```

**¡Con esto deberías poder seleccionar ligandos sin problemas!**

---

## 📞 **Próximos Pasos**

Si después de seguir estos pasos sigues sin poder seleccionar ligandos, ejecuta el diagnóstico y comparte:

1. **Screenshot** del panel de diagnóstico
2. **Console logs** (F12)
3. **Archivos** que estás intentando usar

El nuevo sistema de diagnóstico te dirá exactamente qué está fallando.