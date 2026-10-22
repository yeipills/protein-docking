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
docker compose down 2>/dev/null
pkill -f "vite dev" 2>/dev/null
pkill -f "cargo run" 2>/dev/null

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

# Iniciar todos los servicios con Docker Compose
echo "🚀 Iniciando todos los servicios con Docker Compose..."
docker compose up -d

# Esperar que los servicios estén listos
echo "⏳ Esperando que los servicios estén listos..."
sleep 10

# Verificar servicios
echo "🔍 Verificando servicios..."
docker compose ps

# Mostrar estado final
echo ""
echo "🎉 ¡Protein Docking v2 iniciado exitosamente!"
echo ""
echo "📋 Servicios activos:"
echo "   🌐 Frontend:   http://localhost:3000"
echo "   🔗 Backend:    http://localhost:8080"
echo "   🗄️ ClickHouse: puerto 8123"
echo "   🔴 Redis:      puerto 6379"
echo ""
echo "💡 Para detener todos los servicios:"
echo "   docker compose down"
echo "   # o usar: make stop"
echo ""
echo "📊 Ver logs:"
echo "   docker compose logs -f"