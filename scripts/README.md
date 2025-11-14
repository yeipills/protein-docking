# Scripts de Utilidad

Scripts útiles para operaciones comunes del proyecto.

## Scripts Disponibles

### 🔧 dev-start.sh
Inicia el entorno de desarrollo completo con hot-reload.

**Uso:**
```bash
./scripts/dev-start.sh
```

**Características:**
- Verifica y crea `.env` si no existe
- Detiene contenedores previos
- Construye y levanta todos los servicios en modo desarrollo
- Ejecuta migraciones de base de datos
- Muestra URLs y comandos útiles

**Servicios iniciados:**
- Frontend (http://localhost:3000)
- Backend API (http://localhost:5000)
- API Docs (http://localhost:5000/docs)
- Socket Server (http://localhost:8080)
- PostgreSQL (localhost:5432)
- Redis (localhost:6379)

---

### 💾 backup-db.sh
Crea backups comprimidos de la base de datos PostgreSQL.

**Uso:**
```bash
./scripts/backup-db.sh
```

**Características:**
- Crea backup timestamped en formato SQL
- Comprime con gzip para ahorrar espacio
- Limpia automáticamente backups antiguos (>7 días)
- Muestra comando para restaurar

**Restaurar backup:**
```bash
gunzip -c backups/protein_docking_YYYYMMDD_HHMMSS.sql.gz | \
  docker exec -i protein_docking_postgres psql -U protein_user protein_docking
```

**Ubicación:** `./backups/`

---

### 🚀 deploy-production.sh
Despliega la aplicación a producción con validaciones de seguridad.

**Uso:**
```bash
./scripts/deploy-production.sh
```

**Validaciones pre-deployment:**
- ✓ Verifica que `.env` existe
- ✓ Comprueba que `ENVIRONMENT=production`
- ✓ Detecta passwords y secrets por defecto
- ✓ Advierte si `ALLOWED_ORIGINS` contiene localhost

**Proceso de deployment:**
1. Crea backup de base de datos
2. Pull del código más reciente
3. Construye imágenes Docker (sin caché)
4. Detiene servicios actuales
5. Inicia nuevos servicios
6. Ejecuta migraciones
7. Realiza health checks
8. Muestra estado final

**⚠️ IMPORTANTE:**
Antes de ejecutar en producción:
1. Actualiza `.env` con configuración de producción
2. Cambia todos los passwords y secret keys
3. Configura `ALLOWED_ORIGINS` con tu dominio real
4. Asegúrate de tener backup reciente

---

### 🧪 run-tests.sh
Ejecuta suite completa de tests para backend y frontend.

**Uso:**
```bash
# Ejecutar todos los tests
./scripts/run-tests.sh

# Solo backend
./scripts/run-tests.sh backend

# Solo frontend
./scripts/run-tests.sh frontend

# Solo linting
./scripts/run-tests.sh lint
```

**Tests incluidos:**

**Backend:**
- Pytest con coverage
- Flake8 (linting)
- Mypy (type checking)
- Reporte HTML de cobertura

**Frontend:**
- Vitest (unit tests)
- ESLint (linting)
- Coverage reports

**Reportes generados:**
- Backend: `backend/htmlcov/index.html`
- Frontend: `frontend/coverage/index.html`

---

## Workflow Recomendado

### Desarrollo Local
```bash
# 1. Iniciar entorno de desarrollo
./scripts/dev-start.sh

# 2. Durante desarrollo, ejecutar tests
./scripts/run-tests.sh

# 3. Crear backup antes de cambios mayores
./scripts/backup-db.sh
```

### Pre-Deployment
```bash
# 1. Ejecutar suite completa de tests
./scripts/run-tests.sh

# 2. Verificar que todos pasan
# 3. Crear backup
./scripts/backup-db.sh

# 4. Deploy a producción
./scripts/deploy-production.sh
```

---

## Requisitos

**Todos los scripts requieren:**
- Docker y Docker Compose instalados
- Archivo `.env` configurado
- Permisos de ejecución (ya configurados)

**Scripts específicos:**
- `run-tests.sh` (frontend): Node.js y npm instalados
- `backup-db.sh`: Contenedor PostgreSQL corriendo
- `deploy-production.sh`: Git configurado

---

## Troubleshooting

### Error: "Permission denied"
```bash
chmod +x scripts/*.sh
```

### Error: ".env file not found"
```bash
cp .env.example .env
# Editar .env con tu configuración
```

### Error: "Container not running"
```bash
# Iniciar servicios
docker-compose up -d

# O usar script de desarrollo
./scripts/dev-start.sh
```

### Backup falla con "No such container"
```bash
# Verificar nombre del contenedor
docker ps | grep postgres

# O iniciar PostgreSQL
docker-compose up -d postgres
```

---

## Notas de Seguridad

⚠️ **PRODUCCIÓN:**
- NUNCA uses passwords por defecto en producción
- SIEMPRE cambia `JWT_SECRET_KEY` y `SECRET_KEY`
- SIEMPRE configura `ALLOWED_ORIGINS` con tu dominio real
- Mantén backups en ubicación segura y encriptada
- Revisa logs después de cada deployment

🔒 **BACKUPS:**
- Los backups contienen datos sensibles
- No los commitees al repositorio Git
- Guárdalos en almacenamiento seguro
- Considera encriptación para backups de producción
- Prueba restaurar backups periódicamente

---

## Contribuir

Al agregar nuevos scripts:
1. Usa bash con `set -e` para exit on error
2. Agrega colores para mejor UX
3. Incluye mensajes claros de error
4. Documenta en este README
5. Haz el script ejecutable: `chmod +x scripts/nuevo-script.sh`
