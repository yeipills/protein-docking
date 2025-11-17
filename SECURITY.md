# Security Guide - Protein Docking Platform

> **Version:** 2.4.0
> **Last Updated:** 2025-11-15
> **Status:** Production Security Hardening

## Overview

This document outlines the security architecture, threat model, and best practices for the Protein Docking Platform. Our security strategy implements defense-in-depth with multiple layers of protection.

**Security Posture:**
- ✅ OWASP Top 10 mitigations implemented
- ✅ Enterprise-grade authentication and authorization
- ✅ File upload validation with magic bytes
- ✅ Account lockout and brute force protection
- ✅ XSS, CSRF, and SQL injection prevention
- ✅ Secure JWT implementation with httpOnly cookies

---

## Table of Contents

1. [Security Implementations](#security-implementations)
2. [Authentication & Authorization](#authentication--authorization)
3. [File Upload Security](#file-upload-security)
4. [Input Validation](#input-validation)
5. [API Security](#api-security)
6. [Database Security](#database-security)
7. [Frontend Security](#frontend-security)
8. [Infrastructure Security](#infrastructure-security)
9. [Security Headers](#security-headers)
10. [SSL/TLS Configuration](#ssltls-configuration)
11. [Secrets Management](#secrets-management)
12. [Security Checklist](#security-checklist)

---

## Security Implementations

### Completed Security Measures (v2.2.0 - v2.3.0)

#### 1. XSS Prevention ✅

**Location:** `frontend/src/utils/toast.ts`

**Fix:**
```typescript
// BEFORE (VULNERABLE):
toast.innerHTML = `${icon}<span>${message}</span>`

// AFTER (SECURE):
const messageSpan = document.createElement('span')
messageSpan.textContent = message  // Safe - DOM text node
toast.appendChild(messageSpan)
```

**Impact:** Prevents attackers from injecting malicious scripts

#### 2. JWT httpOnly Cookies ✅

**Location:** `backend/app/api/auth.py`

**Implementation:**
```python
response.set_cookie(
    key="access_token",
    value=access_token,
    httponly=True,       # Not accessible to JavaScript
    secure=is_production,  # HTTPS only in production
    samesite="lax",      # CSRF protection
    max_age=15 * 60      # 15 minute expiry
)
```

**Impact:** Tokens cannot be stolen via XSS attacks

#### 3. Strong Password Policy ✅

**Location:** `backend/app/schemas/user.py`

**Requirements:**
- Minimum 12 characters
- At least 1 uppercase letter
- At least 1 lowercase letter
- At least 1 digit
- At least 1 special character

**Implementation:**
```python
@field_validator('password')
@classmethod
def validate_strong_password(cls, v: str) -> str:
    if len(v) < 12:
        raise ValueError('Password must be at least 12 characters')
    if not re.search(r'[A-Z]', v):
        raise ValueError('Must contain uppercase letter')
    # ... additional checks
    return v
```

#### 4. Account Lockout Policy ✅

**Location:** `backend/app/api/auth.py`

**Policy:**
- Lock after 5 failed login attempts
- Lockout duration: 30 minutes
- Failed attempts reset after 15 minutes

**Impact:** Prevents brute force attacks

#### 5. File Upload Validation ✅

**Location:** `backend/app/core/file_validation.py`

**Validations:**
1. Magic Bytes Checking - Verify actual file type
2. File Size Limits - Prevent resource exhaustion
3. Filename Sanitization - Prevent path traversal
4. MIME Type Verification
5. Malicious File Detection - Block executables, scripts

**Implementation:**
```python
import magic

async def validate_file_upload(file: UploadFile, expected_ext: str):
    # Check extension
    if not file.filename.endswith(expected_ext):
        raise ValueError(f"Invalid extension")

    # Verify magic bytes
    header = await file.read(2048)
    mime = magic.from_buffer(header, mime=True)

    # Block dangerous types
    dangerous = ['application/x-executable', 'application/x-sh']
    if mime in dangerous:
        raise ValueError("File type not allowed")

def sanitize_filename(filename: str) -> str:
    """Prevent path traversal attacks"""
    filename = os.path.basename(filename)
    filename = re.sub(r'[^\w\s\-\.]', '_', filename)
    return filename
```

#### 6. SQL Injection Prevention ✅

**Method:** SQLAlchemy ORM with parameterized queries

```python
# SAFE
user = db.query(User).filter(User.username == username).first()

# NEVER DO THIS
# db.execute(f"SELECT * FROM users WHERE username = '{username}'")
```

#### 7. CSRF Protection ✅

**Method:** SameSite cookie + CORS configuration

```python
response.set_cookie(
    key="access_token",
    samesite="lax"  # Prevents CSRF
)

ALLOWED_ORIGINS = ["http://localhost:3000", "https://yourdomain.com"]
```

---

## Authentication & Authorization

### JWT Token Strategy

**Token Types:**
1. **Access Token** - 15 minutes, httpOnly cookie
2. **Refresh Token** - 7 days, httpOnly cookie

**Token Structure:**
```python
{
    "sub": "user_id",
    "username": "testuser",
    "exp": 1700000000,
    "type": "access"
}
```

**Best Practices:**
- ✅ Strong secret key (64+ characters)
- ✅ Short access token lifetime
- ✅ httpOnly cookies (not localStorage)
- ✅ Verify token on every request

### Role-Based Access Control

**Roles:** `user`, `admin`

```python
@router.get("/admin/users")
async def list_all_users(current_user: User = Depends(require_admin)):
    # Admin-only endpoint
    return db.query(User).all()
```

---

## File Upload Security

### Comprehensive Validation

**Location:** `backend/app/core/file_validation.py`

**Process:**
1. Check file extension
2. Verify magic bytes
3. Validate MIME type
4. Check file size
5. Sanitize filename
6. Scan for malware patterns

**Allowed Files:**
- `.stl` - STL 3D models
- `.vert` - Vertex files
- `.face` - Face files
- `.txt` - Context ray files

**Blocked Files:**
- Executables (.exe, .sh, .bat)
- Scripts (.js, .py, .php)
- Archives (.zip, .tar, .rar)

---

## API Security

### Rate Limiting

**Implementation:** SlowAPI

**Limits:**
- Auth endpoints: 5 req/min
- Upload endpoints: 10 req/min
- Job endpoints: 20 req/hour
- General: 60 req/min

```python
@app.post("/api/v1/auth/login")
@limiter.limit("5/minute")
async def login(...):
    pass
```

### Request Validation

**Headers:** Validate Content-Type, User-Agent, Origin
**Body:** Max 100MB, JSON depth limit 10

---

## Database Security

### PostgreSQL Security

**Connection:**
```python
# Production: Require SSL
DATABASE_URL = "postgresql://user:pass@host:5432/db?sslmode=require"
```

**Best Practices:**
- ✅ SSL/TLS connections
- ✅ Least privilege users
- ✅ Encrypted backups
- ✅ Audit logging

### Password Hashing

**Algorithm:** bcrypt

```python
from passlib.context import CryptContext

pwd_context = CryptContext(schemes=["bcrypt"])

def hash_password(password: str) -> str:
    return pwd_context.hash(password)
```

**Why bcrypt:**
- Adaptive cost factor
- Built-in salt
- Slow by design (prevents brute force)

---

## Frontend Security

### XSS Prevention

**React Automatic Escaping:**
```tsx
// SAFE - React escapes automatically
<div>{userInput}</div>

// UNSAFE - Never use dangerouslySetInnerHTML
<div dangerouslySetInnerHTML={{__html: userInput}} />  // ❌
```

### Content Security Policy

```nginx
add_header Content-Security-Policy "
    default-src 'self';
    script-src 'self' 'unsafe-inline';
    connect-src 'self' wss://yourdomain.com;
    frame-ancestors 'none';
" always;
```

### Dependency Security

```bash
# Check vulnerabilities
npm audit

# Auto-fix
npm audit fix
```

---

## Infrastructure Security

### Nginx Security Headers

```nginx
# Prevent clickjacking
add_header X-Frame-Options "SAMEORIGIN" always;

# XSS protection
add_header X-Content-Type-Options "nosniff" always;

# HTTPS enforcement
add_header Strict-Transport-Security "max-age=31536000" always;

# Hide server version
server_tokens off;
```

### Docker Security

**Best Practices:**
- ✅ Official base images
- ✅ Non-root user
- ✅ Multi-stage builds
- ✅ Scan for vulnerabilities
- ✅ Resource limits

```dockerfile
# Create non-root user
RUN useradd -m appuser
USER appuser
```

---

## Security Headers

### Recommended Headers

```
X-Frame-Options: SAMEORIGIN
X-Content-Type-Options: nosniff
X-XSS-Protection: 1; mode=block
Strict-Transport-Security: max-age=31536000
Content-Security-Policy: default-src 'self'
Referrer-Policy: strict-origin-when-cross-origin
```

---

## SSL/TLS Configuration

### Let's Encrypt (Production)

```bash
# Install certbot
sudo apt-get install certbot python3-certbot-nginx

# Get certificate
sudo certbot --nginx -d yourdomain.com

# Auto-renewal
0 0 * * * certbot renew --quiet
```

### Nginx SSL Configuration

```nginx
server {
    listen 443 ssl http2;

    ssl_certificate /etc/letsencrypt/live/yourdomain.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/yourdomain.com/privkey.pem;

    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_prefer_server_ciphers off;
}

# Redirect HTTP to HTTPS
server {
    listen 80;
    return 301 https://$server_name$request_uri;
}
```

### PostgreSQL SSL

```python
if ENVIRONMENT == "production":
    DATABASE_URL = DATABASE_URL + "?sslmode=require"
```

---

## Secrets Management

### Environment Variables

**Never commit secrets!**

```bash
# Generate strong secrets
openssl rand -hex 32  # 64-character string
```

**.env (local):**
```bash
JWT_SECRET_KEY=your-64-character-random-string
POSTGRES_PASSWORD=strong_password_32_chars_min
```

**Production:**
- Use secrets management (AWS Secrets Manager, Vault)
- Rotate regularly
- Never log secrets

---

## Security Checklist

### Pre-Production

#### Authentication
- [ ] JWT secrets strong (64+ chars)
- [ ] httpOnly cookies enabled
- [ ] Account lockout enabled
- [ ] Strong passwords enforced
- [ ] Session timeout configured

#### Input Validation
- [ ] Pydantic validation
- [ ] File upload validation
- [ ] Filename sanitization
- [ ] SQL injection prevention
- [ ] XSS prevention

#### Infrastructure
- [ ] SSL/TLS configured
- [ ] Security headers enabled
- [ ] CORS configured
- [ ] Rate limiting enabled
- [ ] Firewall rules set

#### Database
- [ ] PostgreSQL SSL enforced
- [ ] Strong password
- [ ] Least privilege permissions
- [ ] Automated backups
- [ ] Backup encryption

#### Secrets
- [ ] No secrets in code
- [ ] Environment variables used
- [ ] .env not committed
- [ ] Regular rotation

#### Monitoring
- [ ] Security logging enabled
- [ ] Alerts configured
- [ ] Log retention set
- [ ] SIEM integration

---

## Security Updates

### Stay Informed

**Resources:**
- OWASP Top 10: https://owasp.org/
- CVE Database: https://cve.mitre.org/
- npm audit / pip-audit

**Regular Tasks:**
- Weekly: Dependency updates
- Monthly: Security audit
- Quarterly: Penetration testing
- Annually: Full review

---

## Contact

**Security Issues:**
Report to: security@yourdomain.com

**Disclosure Policy:**
Responsible disclosure practices

---

**Generated:** 2025-11-15
**Version:** 2.4.0
**Status:** Production Security Guide
