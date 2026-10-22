#!/bin/bash

# Script de inicio para Protein Docking v2
# Inicia todos los servicios necesarios

echo "🧬 Iniciando Protein Docking v2..."

# Verificar dependencias
if ! command -v node &> /dev/null; then
    echo "❌ Node.js no está instalado"
    exit 1
fi

if ! command -v docker &> /dev/null; then
    echo "❌ Docker no está instalado"
    exit 1
fi

# Función para verificar si un puerto está en uso
check_port() {
    if lsof -Pi :$1 -sTCP:LISTEN -t >/dev/null ; then
        return 0
    else
        return 1
    fi
}

# Detener procesos existentes
echo "🔄 Deteniendo procesos existentes..."
pkill -f "mock-backend.js" 2>/dev/null
pkill -f "vite dev" 2>/dev/null
pkill -f "protein-docking-server" 2>/dev/null
pkill -f "server" 2>/dev/null

# Verificar que el puerto 8080 esté libre
if check_port 8080; then
    echo "🔍 Puerto 8080 ocupado, liberando..."
    fuser -k 8080/tcp 2>/dev/null || true
    sleep 1
fi

# Esperar un momento
sleep 2

# Cambiar al directorio del proyecto
cd "$(dirname "$0")"

# Iniciar servicios de base de datos
echo "📊 Iniciando servicios de base de datos..."

# PostgreSQL
if ! check_port 5432; then
    echo "🐘 Iniciando PostgreSQL..."
    docker run -d --name postgres-simple \
        -e POSTGRES_DB=protein_docking \
        -e POSTGRES_USER=protein_user \
        -e POSTGRES_PASSWORD=secure_protein_pass_2024 \
        -p 5432:5432 \
        postgres:15-alpine >/dev/null 2>&1
fi

# Redis Cluster
if ! check_port 6379; then
    echo "🔴 Iniciando Redis Cluster..."
    docker-compose -f docker-compose.unified.yml up -d redis-1 redis-2 redis-3 >/dev/null 2>&1
fi

# Esperar que las bases de datos estén listas
echo "⏳ Esperando que los servicios estén listos..."
sleep 3

# Iniciar Mock Backend
echo "🚀 Iniciando Mock Backend..."
node mock-backend.js &
BACKEND_PID=$!

# Esperar que el backend esté listo
sleep 2

# Verificar backend
if check_port 8080; then
    echo "✅ Backend iniciado en puerto 8080"
else
    echo "❌ Error al iniciar backend"
    exit 1
fi

# Iniciar Frontend
echo "🎨 Iniciando Frontend..."
cd frontend
npm run dev -- --host 0.0.0.0 --port 3000 &
FRONTEND_PID=$!

# Esperar que el frontend esté listo
sleep 5

# Verificar frontend
if check_port 3000; then
    echo "✅ Frontend iniciado en puerto 3000"
else
    echo "❌ Error al iniciar frontend"
    exit 1
fi

# Mostrar estado final
echo ""
echo "🎉 ¡Protein Docking v2 iniciado exitosamente!"
echo ""
echo "📋 Servicios activos:"
echo "   🌐 Frontend:   http://localhost:3000"
echo "   🔗 Backend:    http://localhost:8080"
echo "   🐘 PostgreSQL: puerto 5432"
echo "   🔴 Redis:      puertos 6379-6381"
echo ""
echo "💡 Para detener todos los servicios:"
echo "   ./stop.sh"
echo ""
echo "📊 Logs en tiempo real:"
echo "   tail -f /tmp/protein-docking.log"

# Mantener el script corriendo
echo "🔄 Presiona Ctrl+C para detener todos los servicios..."

# Función de limpieza al salir
cleanup() {
    echo ""
    echo "🛑 Deteniendo servicios..."
    kill $BACKEND_PID 2>/dev/null
    kill $FRONTEND_PID 2>/dev/null
    echo "✅ Servicios detenidos"
}

# Registrar función de limpieza
trap cleanup EXIT

# Esperar indefinidamente
wait