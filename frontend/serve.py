#!/usr/bin/env python3
"""
Simple HTTP server para servir el frontend
Uso: python serve.py [puerto]
"""
import http.server
import socketserver
import sys
import os

# Cambiar al directorio del script
os.chdir(os.path.dirname(os.path.abspath(__file__)))

# Puerto por defecto
PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 8000

# Handler con CORS habilitado
class CORSRequestHandler(http.server.SimpleHTTPRequestHandler):
    def end_headers(self):
        self.send_header('Access-Control-Allow-Origin', '*')
        self.send_header('Access-Control-Allow-Methods', 'GET, POST, OPTIONS')
        self.send_header('Access-Control-Allow-Headers', 'Content-Type, Authorization')
        super().end_headers()

    def do_OPTIONS(self):
        self.send_response(200)
        self.end_headers()

# Crear servidor
with socketserver.TCPServer(("", PORT), CORSRequestHandler) as httpd:
    print(f"🚀 Servidor corriendo en http://localhost:{PORT}")
    print(f"📁 Sirviendo archivos desde: {os.getcwd()}")
    print("🛑 Presiona Ctrl+C para detener")
    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        print("\n👋 Servidor detenido")
        sys.exit(0)
