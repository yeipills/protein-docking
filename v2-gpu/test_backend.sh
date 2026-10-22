#!/bin/bash

# Script para probar el backend funcionando
echo "🧪 Probando Protein Docking Backend v2..."

# Esperar un momento para que el servidor inicie
sleep 2

echo "📍 Probando endpoint raíz..."
curl -s http://localhost:8080/ | head -c 200
echo -e "\n"

echo "🏥 Probando health check..."
curl -s http://localhost:8080/health | head -c 300
echo -e "\n"

echo "🔗 Probando API info..."
curl -s http://localhost:8080/api/v1/ | head -c 200
echo -e "\n"

echo "🏓 Probando ping..."
curl -s http://localhost:8080/api/v1/ping | head -c 150
echo -e "\n"

echo "✅ ¡Pruebas completadas!"