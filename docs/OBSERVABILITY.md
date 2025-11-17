# Observability Stack Documentation

## Table of Contents
- [Overview](#overview)
- [Architecture](#architecture)
- [Quick Start](#quick-start)
- [Prometheus Configuration](#prometheus-configuration)
- [Grafana Dashboards](#grafana-dashboards)
- [Available Metrics](#available-metrics)
- [Alerts and Monitoring](#alerts-and-monitoring)
- [Troubleshooting](#troubleshooting)
- [Production Best Practices](#production-best-practices)

---

## Overview

The Protein Docking Platform includes a comprehensive observability stack for monitoring application performance, tracking metrics, and visualizing system health.

### Stack Components

- **Prometheus** - Time-series database for metrics collection
- **Grafana** - Visualization and dashboarding
- **Postgres Exporter** - Database metrics
- **Redis Exporter** - Cache metrics
- **Application Metrics** - Custom FastAPI metrics via `/metrics` endpoint

### Key Features

✅ Real-time application metrics
✅ HTTP request tracking with latency percentiles
✅ Job processing metrics
✅ Database performance monitoring
✅ Redis cache statistics
✅ Celery task tracking
✅ Pre-configured dashboards
✅ Auto-provisioned data sources

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Grafana (Port 3001)                      │
│              Visualization & Dashboards                     │
└────────────────────┬────────────────────────────────────────┘
                     │ Queries metrics
                     ↓
┌─────────────────────────────────────────────────────────────┐
│                 Prometheus (Port 9090)                      │
│              Time-Series Database                           │
└──┬────────┬──────────┬──────────┬───────────┬──────────────┘
   │        │          │          │           │
   │ Scrapes│          │          │           │
   ↓        ↓          ↓          ↓           ↓
┌──────┐ ┌────┐  ┌──────────┐ ┌──────┐  ┌─────────┐
│Backend│ │PG  │  │  Redis   │ │Celery│  │  Node   │
│ API  │ │Exp.│  │   Exp.   │ │Workers│  │Exporter │
│:5000 │ │:9187│  │  :9121   │ │      │  │ :9100   │
└──────┘ └────┘  └──────────┘ └──────┘  └─────────┘
```

---

## Quick Start

### 1. Start the Monitoring Stack

```bash
# Start main services first
docker-compose up -d

# Start monitoring stack
docker-compose -f docker-compose.yml -f docker-compose.monitoring.yml up -d

# Verify all services are running
docker-compose ps
```

### 2. Access the UIs

**Grafana Dashboard:**
- URL: http://localhost:3001
- Default credentials:
  - Username: `admin`
  - Password: `admin` (change on first login)

**Prometheus UI:**
- URL: http://localhost:9090
- Query metrics directly
- View targets and alerts

**Application Metrics:**
- URL: http://localhost:5000/metrics
- Raw Prometheus format metrics

### 3. View Pre-configured Dashboard

1. Login to Grafana at http://localhost:3001
2. Go to **Dashboards** → **Browse**
3. Open **"Protein Docking - Application Overview"**
4. Set auto-refresh to 30s (top right)

---

## Prometheus Configuration

### Scrape Targets

Prometheus is configured to scrape metrics from:

| Target | Interval | Port | Metrics |
|--------|----------|------|---------|
| Backend API | 10s | 5000 | HTTP, Jobs, Celery, File uploads |
| Prometheus (self) | 15s | 9090 | Prometheus internal metrics |
| Postgres Exporter | 30s | 9187 | Database connections, queries |
| Redis Exporter | 30s | 9121 | Cache hits/misses, memory |

### Configuration File

Location: `prometheus.yml`

**Key sections:**
```yaml
global:
  scrape_interval: 15s  # Default scrape frequency
  evaluation_interval: 15s

scrape_configs:
  - job_name: 'backend-api'
    scrape_interval: 10s  # Override for this job
    static_configs:
      - targets: ['backend:5000']
```

### Reload Configuration

Without restarting:
```bash
# Send SIGHUP to Prometheus
docker-compose kill -s SIGHUP prometheus

# Or use HTTP API (if --web.enable-lifecycle is set)
curl -X POST http://localhost:9090/-/reload
```

---

## Grafana Dashboards

### Pre-configured Dashboards

#### 1. **Protein Docking - Application Overview**

**Panels:**
- **HTTP Request Rate** - Requests/second by endpoint
- **HTTP Latency (P95)** - 95th percentile response time
- **HTTP Status Codes** - Distribution of 2xx, 4xx, 5xx responses
- **Active Jobs** - Currently processing jobs
- **Jobs Created (24h)** - Job creation rate
- **Requests In Progress** - Concurrent requests
- **Job Processing Duration** - P50, P95, P99 latencies
- **Celery Tasks Rate** - Task execution rate by status
- **File Upload Sizes** - Upload size distribution

### Creating Custom Dashboards

1. **From Grafana UI:**
   - Click **+ → Dashboard**
   - Add Panel
   - Select Prometheus datasource
   - Write PromQL query

2. **Example PromQL Queries:**

```promql
# Request rate
rate(http_requests_total[5m])

# P95 latency in milliseconds
histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m])) * 1000

# Active jobs by type
sum by(job_type) (jobs_active)

# Database query duration
histogram_quantile(0.99, rate(db_query_duration_seconds_bucket[5m]))
```

3. **Export Dashboard:**
```bash
# Save dashboard JSON
# Dashboards → Settings → JSON Model → Copy to file
```

### Auto-provisioning

Dashboards in `grafana/dashboards/` are automatically loaded on startup.

To add a new dashboard:
1. Create or export JSON file
2. Place in `grafana/dashboards/`
3. Restart Grafana:
   ```bash
   docker-compose restart grafana
   ```

---

## Available Metrics

### HTTP Metrics

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `http_requests_total` | Counter | method, endpoint, status_code | Total HTTP requests |
| `http_request_duration_seconds` | Histogram | method, endpoint | Request duration |
| `http_requests_in_progress` | Gauge | method, endpoint | Active requests |

**Example Queries:**
```promql
# Error rate (5xx responses)
rate(http_requests_total{status_code=~"5.."}[5m])

# Average request duration
rate(http_request_duration_seconds_sum[5m]) / rate(http_request_duration_seconds_count[5m])
```

### Job Metrics

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `jobs_total` | Counter | job_type, user_id | Jobs created |
| `jobs_completed_total` | Counter | job_type, status | Completed jobs |
| `jobs_active` | Gauge | job_type, status | Active jobs |
| `job_processing_duration_seconds` | Histogram | job_type | Processing time |

**Example Queries:**
```promql
# Job success rate
rate(jobs_completed_total{status="completed"}[5m]) / rate(jobs_total[5m])

# Average job duration
histogram_quantile(0.5, rate(job_processing_duration_seconds_bucket[5m]))
```

### Celery Metrics

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `celery_tasks_total` | Counter | task_name, status | Total tasks |
| `celery_task_duration_seconds` | Histogram | task_name | Task duration |

**Example Queries:**
```promql
# Task failure rate
rate(celery_tasks_total{status="FAILURE"}[5m])

# Slowest tasks (p99)
histogram_quantile(0.99, rate(celery_task_duration_seconds_bucket[5m]))
```

### Database Metrics

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `db_connections_active` | Gauge | - | Active connections |
| `db_query_duration_seconds` | Histogram | operation | Query duration |

**Example Queries:**
```promql
# Connection pool usage
db_connections_active

# Slow queries (p95)
histogram_quantile(0.95, rate(db_query_duration_seconds_bucket[5m]))
```

### File Upload Metrics

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `file_uploads_total` | Counter | file_type, user_id | Total uploads |
| `file_upload_size_bytes` | Histogram | file_type | Upload sizes |

**Example Queries:**
```promql
# Upload rate by type
rate(file_uploads_total[5m])

# Average upload size
histogram_quantile(0.5, rate(file_upload_size_bytes_bucket[5m])) / 1024 / 1024  # MB
```

---

## Alerts and Monitoring

### Setting Up Alerts (Future Enhancement)

Create alert rules in `prometheus/alerts/`:

```yaml
# prometheus/alerts/backend.yml
groups:
  - name: backend_alerts
    interval: 1m
    rules:
      - alert: HighErrorRate
        expr: rate(http_requests_total{status_code=~"5.."}[5m]) > 0.05
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High error rate detected"
          description: "Error rate is {{ $value }} req/s"

      - alert: HighLatency
        expr: histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m])) > 1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High request latency"
          description: "P95 latency is {{ $value }}s"
```

### Recommended Alerts

1. **High Error Rate** - 5xx responses > 5%
2. **High Latency** - P95 > 1 second
3. **Database Connection Pool** - Connections > 80%
4. **Job Queue Backlog** - Pending jobs > 100
5. **Celery Worker Down** - No active workers
6. **Disk Space Low** - Available space < 10%

---

## Troubleshooting

### Prometheus Not Scraping Metrics

**Check target status:**
```bash
# Visit Prometheus UI
http://localhost:9090/targets

# Or via API
curl http://localhost:9090/api/v1/targets
```

**Common issues:**
1. Backend not running: `docker-compose ps backend`
2. Metrics endpoint not exposed: `curl http://localhost:5000/metrics`
3. Network connectivity: `docker network inspect protein-docking-network`

**Fix:**
```bash
# Restart backend
docker-compose restart backend

# Check backend logs
docker-compose logs backend | tail -50
```

### Grafana Dashboard Not Loading

**Check datasource:**
1. Grafana → Configuration → Data Sources
2. Click "Prometheus"
3. Click "Test" - should show "Data source is working"

**If failing:**
```bash
# Check Prometheus is accessible
docker-compose exec grafana wget -O- http://prometheus:9090/api/v1/query?query=up

# Restart Grafana
docker-compose restart grafana
```

### Metrics Not Appearing

**Verify metric generation:**
```bash
# Check /metrics endpoint
curl http://localhost:5000/metrics | grep -A 2 "http_requests_total"

# Should see something like:
# http_requests_total{method="GET",endpoint="/health",status_code="200"} 42.0
```

**If empty:**
- Ensure requests are being made to the API
- Check that middleware is active (see `backend/app/main.py`)

### Exporter Connection Issues

**Postgres Exporter:**
```bash
# Check logs
docker-compose logs postgres-exporter

# Test connection
docker-compose exec postgres-exporter wget -O- http://localhost:9187/metrics
```

**Redis Exporter:**
```bash
# Check logs
docker-compose logs redis-exporter

# Test connection
docker-compose exec redis-exporter wget -O- http://localhost:9121/metrics
```

---

## Production Best Practices

### 1. Data Retention

**Default:** 30 days

**Adjust in `docker-compose.monitoring.yml`:**
```yaml
prometheus:
  command:
    - '--storage.tsdb.retention.time=90d'  # 90 days
    - '--storage.tsdb.retention.size=10GB'  # Or by size
```

### 2. Secure Grafana

**Change default password:**
```yaml
grafana:
  environment:
    - GF_SECURITY_ADMIN_PASSWORD=${GRAFANA_ADMIN_PASSWORD}  # Use env var
```

**Add in `.env`:**
```bash
GRAFANA_ADMIN_PASSWORD=your_secure_password_here
```

**Disable anonymous access:**
```yaml
grafana:
  environment:
    - GF_AUTH_ANONYMOUS_ENABLED=false
```

### 3. Resource Limits

**Add to `docker-compose.monitoring.yml`:**
```yaml
prometheus:
  deploy:
    resources:
      limits:
        cpus: '2.0'
        memory: 2G
      reservations:
        cpus: '0.5'
        memory: 512M

grafana:
  deploy:
    resources:
      limits:
        cpus: '1.0'
        memory: 512M
```

### 4. Backup Grafana Dashboards

```bash
# Export all dashboards
docker-compose exec grafana grafana-cli admin export-dashboard > backup.json

# Or manually:
# Dashboards → Settings → JSON Model → Copy
```

### 5. External Access

**Use reverse proxy (Nginx/Traefik):**
```nginx
# Grafana
location /grafana/ {
    proxy_pass http://localhost:3001/;
    proxy_set_header Host $host;
}

# Prometheus (restrict access!)
location /prometheus/ {
    auth_basic "Prometheus";
    auth_basic_user_file /etc/nginx/.htpasswd;
    proxy_pass http://localhost:9090/;
}
```

### 6. Enable HTTPS

**With Let's Encrypt:**
```yaml
grafana:
  environment:
    - GF_SERVER_PROTOCOL=https
    - GF_SERVER_CERT_FILE=/etc/grafana/ssl/cert.pem
    - GF_SERVER_CERT_KEY=/etc/grafana/ssl/key.pem
  volumes:
    - ./ssl:/etc/grafana/ssl:ro
```

---

## Performance Tuning

### Prometheus

**Reduce scrape intervals for non-critical metrics:**
```yaml
scrape_configs:
  - job_name: 'backend-api'
    scrape_interval: 10s  # Critical

  - job_name: 'postgres'
    scrape_interval: 60s  # Less critical
```

**Limit metric cardinality:**
```python
# Avoid high-cardinality labels like user_id in production
# Instead, aggregate by user tier or role
jobs_total.labels(job_type=job_type, user_tier="premium")
```

### Grafana

**Use query caching:**
```yaml
grafana:
  environment:
    - GF_DATAPROXY_TIMEOUT=300
    - GF_DATAPROXY_KEEP_ALIVE_SECONDS=300
```

**Limit dashboard refresh:**
- Set minimum refresh to 30s
- Avoid setting to 5s or less

---

## Quick Reference

### Common Commands

```bash
# Start monitoring stack
docker-compose -f docker-compose.yml -f docker-compose.monitoring.yml up -d

# Stop monitoring stack
docker-compose -f docker-compose.monitoring.yml down

# View logs
docker-compose logs -f prometheus
docker-compose logs -f grafana

# Restart service
docker-compose restart prometheus
docker-compose restart grafana

# Execute PromQL query
curl 'http://localhost:9090/api/v1/query?query=up'

# Check metrics endpoint
curl http://localhost:5000/metrics
```

### Useful URLs

- Grafana: http://localhost:3001
- Prometheus: http://localhost:9090
- Backend Metrics: http://localhost:5000/metrics
- Prometheus Targets: http://localhost:9090/targets
- Prometheus Alerts: http://localhost:9090/alerts

---

## Support

For issues:
1. Check [Troubleshooting](#troubleshooting) section
2. Review Docker logs: `docker-compose logs [service]`
3. Verify network connectivity
4. Check Prometheus targets status

---

**Last Updated:** 2025-11-14
**Version:** 1.0
**Maintained by:** yeipills
