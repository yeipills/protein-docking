# 🐳 Docker Setup - Protein Docking Platform

Guía completa para ejecutar la plataforma usando Docker.

## 📋 Requisitos Previos

- Docker 20.10+
- Docker Compose 2.0+
- 4GB RAM mínimo (8GB recomendado)
- 10GB espacio en disco

## 🚀 Inicio Rápido

### 1. Configurar Variables de Entorno

```bash
# Copiar archivo de ejemplo
cp .env.example .env

# Editar variables de producción
nano .env
```

**Variables críticas a cambiar en producción:**
```env
# Cambiar estos valores obligatoriamente:
POSTGRES_PASSWORD=tu_password_seguro_aqui
JWT_SECRET_KEY=tu_jwt_secret_minimo_64_caracteres_muy_largo_y_aleatorio
SECRET_KEY=tu_secret_key_para_encriptacion_general
SOCKET_SECRET_KEY=tu_socket_secret_key_minimo_32_caracteres_random
```

### 2. Producción

```bash
# Construir imágenes
docker-compose build

# Iniciar todos los servicios
docker-compose up -d

# Ver logs
docker-compose logs -f

# Verificar estado
docker-compose ps
```

**La aplicación estará disponible en:**
- Frontend: http://localhost
- Backend API: http://localhost/api/v1
- Socket.IO: http://localhost/socket.io
- Documentación API: http://localhost/api/v1/docs

### 3. Desarrollo

```bash
# Iniciar en modo desarrollo con hot-reload
docker-compose -f docker-compose.dev.yml up -d

# Ver logs del frontend (con HMR)
docker-compose -f docker-compose.dev.yml logs -f frontend

# Ver logs del backend
docker-compose -f docker-compose.dev.yml logs -f backend
```

**Puertos en desarrollo:**
- Frontend: http://localhost:3000 (Vite dev server con HMR)
- Backend: http://localhost:5000
- Socket.IO: http://localhost:8080
- PostgreSQL: localhost:5432
- Redis: localhost:6379

## 🏗️ Arquitectura de Servicios

```
┌─────────────┐
│   Nginx     │ :80, :443
│  (Reverse   │
│   Proxy)    │
└──────┬──────┘
       │
   ┌───┴────┬──────────┬──────────┐
   │        │          │          │
┌──▼───┐ ┌─▼──────┐ ┌─▼──────┐ ┌─▼────────┐
│Front │ │Backend │ │ Socket │ │  Celery  │
│ end  │ │  API   │ │  IO    │ │  Worker  │
│React │ │FastAPI │ │ Flask  │ │ (Tasks)  │
└──────┘ └────┬───┘ └────┬───┘ └────┬─────┘
              │          │          │
         ┌────┴──────────┴──────────┴───┐
         │                               │
    ┌────▼─────┐                   ┌────▼────┐
    │PostgreSQL│                   │  Redis  │
    │    DB    │                   │ Cache   │
    └──────────┘                   └─────────┘
```

### Servicios Incluidos

1. **PostgreSQL** - Base de datos principal
   - Puerto: 5432
   - Volumen: `postgres_data`
   - Healthcheck: pg_isready

2. **Redis** - Cache y message broker
   - Puerto: 6379
   - Volumen: `redis_data`
   - Persistencia: appendonly

3. **Backend FastAPI** - API REST
   - Puerto: 5000
   - Workers: 4 (producción), 1 con reload (desarrollo)
   - Cython compilado para máximo rendimiento

4. **Socket.IO Server** - WebSocket real-time
   - Puerto: 8080
   - Auto-reconexión configurada

5. **Celery Worker** - Procesamiento asíncrono
   - Concurrency: 2 (producción), 1 (desarrollo)
   - Conectado a Redis

6. **Frontend React** - Interfaz de usuario
   - Puerto: 3000
   - Producción: Nginx sirviendo build estático
   - Desarrollo: Vite dev server con HMR

7. **Nginx** - Reverse proxy
   - Puertos: 80, 443
   - Rate limiting configurado
   - WebSocket proxy para Socket.IO

## 📦 Volúmenes Persistentes

```bash
# Ver volúmenes
docker volume ls | grep protein

# Backup de la base de datos
docker-compose exec postgres pg_dump -U protein_user protein_docking > backup.sql

# Restaurar base de datos
cat backup.sql | docker-compose exec -T postgres psql -U protein_user protein_docking

# Limpiar volúmenes (⚠️ CUIDADO: Borra todos los datos)
docker-compose down -v
```

## 🔧 Comandos Útiles

### Gestión de Servicios

```bash
# Detener todos los servicios
docker-compose down

# Reiniciar un servicio específico
docker-compose restart backend

# Ver logs de un servicio
docker-compose logs -f backend

# Entrar a un contenedor
docker-compose exec backend bash
docker-compose exec postgres psql -U protein_user protein_docking

# Reconstruir un servicio
docker-compose build --no-cache frontend
docker-compose up -d frontend
```

### Migraciones de Base de Datos

```bash
# Aplicar migraciones
docker-compose exec backend alembic upgrade head

# Crear nueva migración
docker-compose exec backend alembic revision --autogenerate -m "descripcion"

# Ver historial
docker-compose exec backend alembic history
```

### Celery Worker

```bash
# Ver tareas activas
docker-compose exec celery_worker celery -A app.tasks.celery_app inspect active

# Ver tareas programadas
docker-compose exec celery_worker celery -A app.tasks.celery_app inspect scheduled

# Purgar todas las tareas
docker-compose exec celery_worker celery -A app.tasks.celery_app purge
```

### Frontend

```bash
# Desarrollo - Acceder al contenedor
docker-compose -f docker-compose.dev.yml exec frontend sh

# Instalar nueva dependencia
docker-compose -f docker-compose.dev.yml exec frontend npm install nombre-paquete

# Reconstruir (si cambia package.json)
docker-compose -f docker-compose.dev.yml build frontend
docker-compose -f docker-compose.dev.yml up -d frontend

# Producción - Ver build optimizado
docker-compose exec frontend ls -lah /usr/share/nginx/html
```

## 🐛 Troubleshooting

### Frontend no carga

```bash
# Verificar logs
docker-compose logs frontend

# Verificar que el build se completó
docker-compose exec frontend ls -la /usr/share/nginx/html

# Reconstruir
docker-compose build --no-cache frontend
docker-compose up -d frontend
```

### Backend no conecta a PostgreSQL

```bash
# Verificar que postgres está healthy
docker-compose ps postgres

# Verificar conexión
docker-compose exec backend python -c "from app.database import engine; print(engine)"

# Ver logs de postgres
docker-compose logs postgres
```

### Celery no procesa trabajos

```bash
# Verificar worker está activo
docker-compose ps celery_worker

# Ver logs
docker-compose logs -f celery_worker

# Reiniciar worker
docker-compose restart celery_worker redis
```

### Socket.IO no conecta

```bash
# Verificar logs
docker-compose logs socket

# Testear WebSocket manualmente
wscat -c ws://localhost:8080/socket.io/?transport=websocket

# Verificar Nginx proxy
docker-compose exec nginx cat /etc/nginx/conf.d/nginx.conf | grep socket
```

### Limpiar todo y empezar de nuevo

```bash
# ⚠️ CUIDADO: Esto borra TODOS los datos
docker-compose down -v
docker system prune -a --volumes
docker-compose up -d --build
```

## 🔒 Seguridad en Producción

### Checklist antes de desplegar:

- [ ] Cambiar todas las contraseñas y secrets en `.env`
- [ ] Configurar JWT_SECRET_KEY con mínimo 64 caracteres aleatorios
- [ ] Configurar ALLOWED_ORIGINS con tu dominio real
- [ ] Activar HTTPS en Nginx (configurar certificados SSL)
- [ ] Configurar límites de rate limiting según tu caso de uso
- [ ] Revisar logs y configurar rotación
- [ ] Configurar backups automáticos de PostgreSQL
- [ ] Cambiar `BACKEND_RELOAD=false` en producción
- [ ] Configurar SENTRY_DSN para monitoreo de errores
- [ ] Revisar permisos de volúmenes

### Configurar HTTPS (Nginx)

1. Agregar certificados SSL a `./nginx/ssl/`
2. Actualizar `nginx/nginx.conf`:

```nginx
server {
    listen 443 ssl http2;
    ssl_certificate /etc/nginx/ssl/cert.pem;
    ssl_certificate_key /etc/nginx/ssl/key.pem;
    # ... resto de la configuración
}
```

3. Actualizar `docker-compose.yml`:

```yaml
nginx:
  volumes:
    - ./nginx/ssl:/etc/nginx/ssl:ro
```

## 📊 Monitoreo

### Healthchecks

```bash
# Backend
curl http://localhost:5000/health

# Frontend
curl http://localhost:3000

# Nginx
curl http://localhost/health
```

### Recursos

```bash
# Ver uso de recursos
docker stats

# Ver tamaño de imágenes
docker images | grep protein_docking

# Ver uso de volúmenes
docker system df -v
```

## 🚢 Despliegue en Producción

### Opción 1: VPS/Servidor Dedicado

```bash
# 1. Clonar repositorio
git clone https://github.com/yeipills/protein-docking.git
cd protein-docking

# 2. Configurar .env
cp .env.example .env
nano .env  # Cambiar todos los secrets

# 3. Construir y levantar
docker-compose build
docker-compose up -d

# 4. Verificar
docker-compose ps
curl http://localhost/health
```

### Opción 2: Docker Swarm

```bash
# Inicializar swarm
docker swarm init

# Desplegar stack
docker stack deploy -c docker-compose.yml protein-docking

# Ver servicios
docker service ls
```

### Opción 3: Kubernetes

Ver `k8s/` (si existe) o contactar al equipo de DevOps.

## 📝 Notas Adicionales

- **Hot-reload en desarrollo**: Los cambios en `frontend/src/` se reflejan automáticamente
- **Compilación Cython**: El backend usa Cython compilado para 4-6x más velocidad
- **Multi-stage builds**: Imágenes optimizadas (~200MB frontend, ~400MB backend)
- **Healthchecks**: Todos los servicios tienen healthchecks configurados
- **Auto-restart**: `restart: unless-stopped` en todos los servicios

## 🆘 Soporte

Si encuentras problemas:
1. Revisa logs: `docker-compose logs -f [servicio]`
2. Verifica healthchecks: `docker-compose ps`
3. Consulta la documentación: `/docs` en el backend
4. Abre un issue en GitHub

---

**Última actualización**: 2025-01-XX
**Versión de Docker Compose**: 3.8
**Versión de la plataforma**: 2.0.0
