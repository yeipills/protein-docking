# 🚀 Setup Guide - Protein Docking Platform

Guía paso a paso para ejecutar la plataforma en tu máquina local.

## 📋 Requisitos Previos

Antes de comenzar, asegúrate de tener instalado:

- **Docker** >= 20.10 ([Instalar Docker](https://docs.docker.com/get-docker/))
- **Docker Compose** >= 2.0 ([Instalar Docker Compose](https://docs.docker.com/compose/install/))
- **Git** ([Instalar Git](https://git-scm.com/downloads))
- **Node.js 20+** (solo para desarrollo frontend sin Docker) ([Instalar Node](https://nodejs.org/))

Verifica las versiones:
```bash
docker --version
docker-compose --version
git --version
node --version  # Opcional
```

---

## ⚡ Inicio Rápido (30 minutos)

### 1️⃣ Clonar el Repositorio

```bash
git clone https://github.com/yeipills/protein-docking.git
cd protein-docking
```

### 2️⃣ Configurar Variables de Entorno

**Root `.env`** (Backend, DB, Redis):
```bash
cp .env.example .env
```

Editar `.env` y cambiar estos valores **OBLIGATORIOS**:
```bash
# Abrir con tu editor favorito
nano .env
# o
code .env
```

**Valores a cambiar**:
```env
# ⚠️ CRÍTICO: Cambiar en producción
POSTGRES_PASSWORD=tu_password_seguro_aqui_minimo_16_caracteres
JWT_SECRET_KEY=tu_jwt_secret_minimo_64_caracteres_muy_largo_y_completamente_aleatorio
SECRET_KEY=tu_secret_key_general_minimo_32_caracteres_aleatorios
SOCKET_SECRET_KEY=tu_socket_secret_minimo_32_caracteres_random

# Opcional: Cambiar si usas puertos diferentes
POSTGRES_PORT=5432
REDIS_PORT=6379
```

**Frontend `.env`** (Opcional - solo si cambias URLs):
```bash
cd frontend
cp .env.example .env
cd ..
```

Por defecto está configurado correctamente. Solo editar si cambias puertos.

### 3️⃣ Iniciar con Docker (Opción Recomendada)

**✨ Opción A - Script Automatizado (Recomendado):**
```bash
./scripts/dev-start.sh
```

Este script:
- ✅ Verifica configuración
- ✅ Construye e inicia todos los servicios
- ✅ Ejecuta migraciones automáticamente
- ✅ Muestra URLs y comandos útiles

**Opción B - Manual:**

**Modo Desarrollo** (con hot-reload):
```bash
docker-compose -f docker-compose.dev.yml up -d --build
```

**Modo Producción**:
```bash
docker-compose up -d --build
```

Esto iniciará 7 servicios:
- ✅ PostgreSQL (base de datos)
- ✅ Redis (cache y message broker)
- ✅ Backend FastAPI (API)
- ✅ Socket.IO Server (WebSocket)
- ✅ Celery Worker (procesamiento async)
- ✅ Frontend React (UI)
- ✅ Nginx (reverse proxy)

**Ver logs en tiempo real**:
```bash
docker-compose logs -f

# Ver logs de un servicio específico
docker-compose logs -f backend
docker-compose logs -f frontend
```

### 4️⃣ Ejecutar Migraciones de Base de Datos

```bash
# Crear esquema de base de datos
docker-compose exec backend alembic upgrade head
```

### 5️⃣ Crear Usuario Admin (Opcional)

```bash
docker-compose exec backend python -c "
from app.database import SessionLocal
from app.core.security import get_password_hash
from app.models.user import User

db = SessionLocal()

# Verificar si ya existe
existing = db.query(User).filter(User.email == 'admin@example.com').first()
if existing:
    print('❌ Admin user already exists!')
else:
    admin = User(
        email='admin@example.com',
        username='admin',
        hashed_password=get_password_hash('admin123'),
        is_active=True,
        is_superuser=True
    )
    db.add(admin)
    db.commit()
    print('✅ Admin user created successfully!')
    print('   Email: admin@example.com')
    print('   Password: admin123')
    print('   ⚠️  Change password after first login!')

db.close()
"
```

### 6️⃣ Acceder a la Aplicación

Abre tu navegador en:

- **Frontend**: http://localhost:3000
- **Backend API Docs**: http://localhost:5000/docs
- **Nginx (Producción)**: http://localhost

**Credenciales de prueba**:
- Email: `admin@example.com`
- Password: `admin123`

---

## 🛠️ Desarrollo Frontend (Sin Docker)

Si prefieres desarrollar el frontend sin Docker:

### 1. Instalar Dependencias

```bash
cd frontend
npm install
```

Esto creará:
- `node_modules/` (~200MB)
- `package-lock.json`

### 2. Iniciar Dev Server

```bash
npm run dev
```

El frontend estará en: http://localhost:3000

**Hot Module Replacement (HMR)**: Los cambios se reflejan instantáneamente.

### 3. Build para Producción

```bash
npm run build

# Ver build localmente
npm run preview
```

Build generado en: `dist/` (~150KB gzipped)

---

## 🔍 Verificación de Instalación

### Verificar Servicios

```bash
# Ver estado de todos los servicios
docker-compose ps

# Debe mostrar 7 servicios "Up"
```

### Healthchecks

```bash
# Backend API
curl http://localhost:5000/health
# Esperado: {"status":"healthy"}

# Frontend
curl http://localhost:3000
# Esperado: HTML de la app

# PostgreSQL
docker-compose exec postgres pg_isready -U protein_user
# Esperado: accepting connections

# Redis
docker-compose exec redis redis-cli ping
# Esperado: PONG
```

### Ver Base de Datos

```bash
# Conectar a PostgreSQL
docker-compose exec postgres psql -U protein_user -d protein_docking

# Comandos útiles:
\dt              # Listar tablas
\d users         # Ver estructura de tabla users
SELECT * FROM users;  # Ver usuarios
\q               # Salir
```

---

## 🐛 Troubleshooting

### Error: "Port already in use"

```bash
# Ver qué está usando el puerto
lsof -i :5000  # Backend
lsof -i :3000  # Frontend
lsof -i :5432  # PostgreSQL

# Detener servicios
docker-compose down

# O cambiar puerto en .env
BACKEND_PORT=5001  # Por ejemplo
```

### Error: "Cannot connect to PostgreSQL"

```bash
# Ver logs de postgres
docker-compose logs postgres

# Reiniciar postgres
docker-compose restart postgres

# Verificar si postgres está healthy
docker-compose ps postgres
```

### Error: "Frontend can't connect to API"

Verificar CORS y URLs:
```bash
# En .env root
ALLOWED_ORIGINS=http://localhost:3000,http://localhost:80

# En frontend/.env
VITE_API_URL=http://localhost:5000/api/v1
VITE_SOCKET_URL=http://localhost:8080
```

### Error: "node_modules not found"

```bash
cd frontend
npm install
```

### Limpiar y Reiniciar Todo

```bash
# ⚠️ CUIDADO: Esto borra TODOS los datos
docker-compose down -v
docker system prune -a --volumes

# Volver a iniciar
docker-compose up -d --build
docker-compose exec backend alembic upgrade head
```

---

## 📁 Estructura de Directorios

Después del setup, deberías tener:

```
protein-docking/
├── .env                       # ✅ TU CONFIGURACIÓN
├── docker-compose.yml         # ✅ Compose producción
├── docker-compose.dev.yml     # ✅ Compose desarrollo
│
├── backend/
│   ├── app/                   # ✅ Código Python
│   ├── requirements.txt       # ✅ Dependencias
│   └── Dockerfile             # ✅ Docker backend
│
├── frontend/
│   ├── node_modules/          # ✅ Después de npm install
│   ├── package-lock.json      # ✅ Después de npm install
│   ├── dist/                  # ✅ Después de npm run build
│   ├── src/                   # ✅ Código React
│   ├── public/                # ✅ Assets estáticos
│   │   ├── protein-icon.svg   # ✅ Favicon
│   │   └── robots.txt         # ✅ SEO
│   ├── .eslintrc.cjs          # ✅ ESLint config
│   ├── .prettierrc            # ✅ Prettier config
│   └── .env                   # ✅ TU CONFIGURACIÓN (opcional)
│
├── scripts/                   # ✅ Scripts de utilidad
│   ├── dev-start.sh           # ✅ Inicio rápido desarrollo
│   ├── deploy-production.sh   # ✅ Deploy a producción
│   ├── backup-db.sh           # ✅ Backup automático
│   ├── run-tests.sh           # ✅ Tests automatizados
│   └── README.md              # ✅ Documentación scripts
│
└── nginx/
    ├── nginx.conf             # ✅ Reverse proxy
    └── Dockerfile             # ✅ Docker nginx
```

---

## 🔐 Seguridad - Checklist Pre-Producción

Antes de desplegar a producción, verifica:

- [ ] Cambiaste **TODOS** los secrets en `.env`
- [ ] `JWT_SECRET_KEY` tiene mínimo 64 caracteres aleatorios
- [ ] `POSTGRES_PASSWORD` es fuerte (16+ caracteres)
- [ ] `ENVIRONMENT=production` en `.env`
- [ ] `ALLOWED_ORIGINS` configurado con tu dominio (NO localhost)
- [ ] `VITE_API_URL` y `VITE_SOCKET_URL` configurados con tu dominio
- [ ] Configuraste SSL/HTTPS (Let's Encrypt)
- [ ] Configuraste backups automáticos de PostgreSQL (usa `./scripts/backup-db.sh`)
- [ ] Configuraste firewall (solo puertos 80, 443 abiertos)
- [ ] Revisaste logs para errores
- [ ] Cambiaste password de usuario admin
- [ ] Configuraste límites de rate limiting según tu caso

---

## 📊 Comandos Útiles

### Docker

```bash
# Ver logs en tiempo real
docker-compose logs -f [service_name]

# Reiniciar un servicio
docker-compose restart backend

# Detener todos los servicios
docker-compose down

# Ver uso de recursos
docker stats

# Acceder a un contenedor
docker-compose exec backend bash
```

### Base de Datos

```bash
# Backup (script automatizado - recomendado)
./scripts/backup-db.sh

# Backup manual
docker-compose exec postgres pg_dump -U protein_user protein_docking > backup.sql

# Restore
cat backup.sql | docker-compose exec -T postgres psql -U protein_user protein_docking

# Crear migración
docker-compose exec backend alembic revision --autogenerate -m "descripcion"

# Aplicar migraciones
docker-compose exec backend alembic upgrade head
```

### Celery

```bash
# Ver tareas activas
docker-compose exec celery_worker celery -A app.tasks.celery_app inspect active

# Purgar todas las tareas
docker-compose exec celery_worker celery -A app.tasks.celery_app purge
```

---

## 🎓 Próximos Pasos

Después del setup:

1. **Leer la documentación**:
   - `README.md` - Visión general
   - `DOCKER.md` - Guía detallada de Docker
   - `scripts/README.md` - Scripts de utilidad
   - `ALGORITHMS_STATUS.md` - Info sobre algoritmos

2. **Explorar la API**:
   - Ir a http://localhost:5000/docs
   - Probar endpoints interactivamente

3. **Testear el frontend**:
   - Registrar un usuario
   - Subir una proteína de prueba
   - Ver el progreso en tiempo real

4. **Configurar monitoreo** (opcional):
   - Setup de Prometheus + Grafana
   - Integración con Sentry

---

## 🆘 Ayuda

Si tienes problemas:

1. **Revisa logs**: `docker-compose logs -f`
2. **Verifica healthchecks**: Ejecuta los comandos de verificación arriba
3. **Consulta troubleshooting**: Ver sección arriba
4. **Lee la documentación**: `README.md`, `DOCKER.md`
5. **Abre un issue**: https://github.com/yeipills/protein-docking/issues

---

**¡Listo! Tu plataforma de Protein Docking está corriendo** 🎉

Para detener:
```bash
docker-compose down
```

Para reiniciar:
```bash
docker-compose up -d
```
