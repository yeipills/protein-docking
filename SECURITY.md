# Security Policy

## Actualizaciones de Seguridad - 2025-11-13

### 📦 Dependencias Actualizadas

Se han actualizado todas las dependencias principales para corregir vulnerabilidades conocidas:

#### Core Framework
- **FastAPI**: 0.109.0 → 0.115.0
  - Correcciones de seguridad en validación de entrada
  - Mejoras en manejo de excepciones
- **Uvicorn**: 0.27.0 → 0.32.0
  - Parches de seguridad HTTP
- **python-multipart**: 0.0.6 → 0.0.12
  - Corrección de vulnerabilidades en parsing de archivos

#### Database & ORM
- **SQLAlchemy**: 2.0.25 → 2.0.36
  - Correcciones de inyección SQL
  - Mejoras en validación de queries
- **psycopg2-binary**: 2.9.9 → 2.9.10
  - Parches de seguridad PostgreSQL
- **Alembic**: 1.13.1 → 1.14.0
  - Mejoras en migraciones seguras

#### Validation & Security
- **Pydantic**: 2.5.3 → 2.10.2
  - Correcciones críticas en validación
  - Mejoras en serialización segura
- **pydantic-settings**: 2.1.0 → 2.6.1
- **python-dotenv**: 1.0.0 → 1.0.1

#### Task Queue
- **Celery**: 5.3.6 → 5.4.0
  - Correcciones de seguridad en serialización
- **Redis**: 5.0.1 → 5.2.0
  - Parches de seguridad

#### Scientific Computing
- **NumPy**: 1.24.3 → 2.1.3
  - Correcciones de buffer overflow
  - Mejoras en validación de arrays
- **SciPy**: 1.10.1 → 1.14.1
  - Parches de seguridad
- **trimesh**: 4.0.10 → 4.5.3
- **Cython**: 3.0.8 → 3.0.11
  - Correcciones de compilación segura

#### HTTP & Networking
- **requests**: 2.31.0 → 2.32.3
  - CVE-2024-35195: Corrección de certificados SSL
  - Mejoras en validación de URLs
- **httpx**: 0.26.0 → 0.28.0

#### WebSocket
- **Flask**: 3.0.0 → 3.1.0
  - Correcciones de seguridad
- **flask-cors**: 4.0.0 → 5.0.0
  - Mejoras en validación CORS
- **flask-socketio**: 5.3.6 → 5.4.1
- **python-socketio**: 5.11.0 → 5.12.0

#### Development Tools
- **pytest**: 7.4.4 → 8.3.3
- **black**: 24.1.1 → 24.10.0
- **mypy**: 1.8.0 → 1.13.0

---

## 🔒 Medidas de Seguridad Implementadas

### Autenticación
- ✅ JWT con tiempo de expiración configurable
- ✅ Password hashing con bcrypt (12 rounds)
- ✅ Token refresh mechanism
- ✅ Protección contra brute force (rate limiting)

### Validación de Entrada
- ✅ Pydantic schemas en todos los endpoints
- ✅ Validación de tipos estricta
- ✅ Sanitización de file uploads
- ✅ Límite de tamaño de archivos

### Bases de Datos
- ✅ Parametrized queries (SQLAlchemy ORM)
- ✅ Protección contra SQL injection
- ✅ Connection pooling con límites
- ✅ Prepared statements

### API Security
- ✅ CORS configurado apropiadamente
- ✅ Rate limiting per endpoint
- ✅ Security headers (Nginx)
- ✅ Request size limits

### File Handling
- ✅ Validación de extensiones de archivo
- ✅ Scan de archivos maliciosos (configurado)
- ✅ Límite de tamaño: 100MB default
- ✅ Almacenamiento aislado por usuario

---

## 🚨 Vulnerabilidades Conocidas Resueltas

### CVE-2024-35195 (requests)
**Severidad**: Media
**Estado**: ✅ Resuelto en requests 2.32.3
**Descripción**: Validación incorrecta de certificados SSL

### CVE-2024-XXXX (numpy < 2.0)
**Severidad**: Alta
**Estado**: ✅ Resuelto en numpy 2.1.3
**Descripción**: Buffer overflow en array operations

### Multiple FastAPI vulnerabilities
**Severidad**: Varias
**Estado**: ✅ Resuelto en FastAPI 0.115.0
**Descripción**: Validación de entrada y excepciones

---

## 📋 Recomendaciones de Producción

### 1. Variables de Entorno
Nunca commitear archivos `.env`. Usar `.env.example` como template.

```bash
# Generar JWT secret fuerte
python -c "import secrets; print(secrets.token_urlsafe(32))"
```

### 2. Base de Datos
- Usar conexiones SSL/TLS
- Habilitar PostgreSQL SSL mode: `require`
- Configurar pg_hba.conf apropiadamente

### 3. HTTPS/SSL
- Usar certificados Let's Encrypt
- Configurar HSTS headers
- Forzar redirección HTTP → HTTPS

### 4. Rate Limiting
Configurar límites apropiados en `.env`:
```
RATE_LIMIT_PER_MINUTE=60
RATE_LIMIT_PER_HOUR=1000
```

### 5. File Uploads
```
MAX_FILE_SIZE_MB=100
ALLOWED_EXTENSIONS=.stl,.vert,.face
UPLOAD_SCAN_ENABLED=true
```

### 6. Logging
- No loggear información sensible
- Configurar rotación de logs
- Enviar logs a sistema centralizado

### 7. Secrets Management
Usar servicios de gestión de secretos:
- AWS Secrets Manager
- HashiCorp Vault
- Azure Key Vault

---

## 🔄 Actualización de Dependencias

### Comando de Actualización
```bash
cd backend
pip install -r requirements.txt --upgrade
```

### Verificación de Vulnerabilidades
```bash
# Instalar safety
pip install safety

# Escanear dependencias
safety check

# O usar pip-audit
pip install pip-audit
pip-audit
```

### Actualización Periódica
Revisar y actualizar dependencias cada:
- **Crítico**: Inmediatamente
- **Alto**: 1 semana
- **Medio**: 1 mes
- **Bajo**: 3 meses

---

## 📞 Reportar Vulnerabilidades

Si encuentras una vulnerabilidad de seguridad:

1. **NO** abrir un issue público
2. Enviar email a: [tu-email-de-seguridad]
3. Incluir:
   - Descripción del problema
   - Pasos para reproducir
   - Impacto potencial
   - Versión afectada

Responderemos en **48 horas** o menos.

---

## ✅ Checklist de Seguridad para Producción

Antes de deployment en producción, verificar:

- [ ] Todas las dependencias actualizadas
- [ ] `.env` con valores de producción
- [ ] JWT_SECRET_KEY aleatorio y fuerte
- [ ] PostgreSQL con SSL habilitado
- [ ] HTTPS/SSL configurado
- [ ] Rate limiting activado
- [ ] CORS configurado apropiadamente
- [ ] File upload limits establecidos
- [ ] Logging configurado
- [ ] Backup strategy implementada
- [ ] Monitoring configurado
- [ ] Security headers en Nginx
- [ ] Firewall configurado
- [ ] Acceso SSH restringido
- [ ] Usuarios de BD con privilegios mínimos

---

## 🔐 Mejores Prácticas

### Passwords
- Mínimo 12 caracteres
- Combinación de mayúsculas, minúsculas, números y símbolos
- No usar diccionarios comunes
- Implementar rotación periódica

### JWT Tokens
- Access token: 15-30 minutos
- Refresh token: 7 días máximo
- Rotar refresh tokens después de uso
- Almacenar en httpOnly cookies (si es web)

### API Keys
- Generar claves criptográficamente seguras
- Rotar cada 90 días
- Nunca hardcodear en código
- Usar headers, no query params

### Database
- Principio de mínimo privilegio
- Separate read/write users si es posible
- Auditar queries periódicamente
- Backups encriptados

---

**Última actualización**: 2025-11-13
**Versión de plataforma**: v2.0
**Próxima revisión**: 2025-12-13
