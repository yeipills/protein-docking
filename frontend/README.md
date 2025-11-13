# Frontend - Protein Docking Platform

Interfaz web simple para la plataforma de protein docking.

## Características

- 🔐 **Autenticación**: Login y registro de usuarios
- 📊 **Dashboard**: Visualización de trabajos y progreso en tiempo real
- ⬆️ **Upload**: Subida de archivos de proteínas (STL, VERT, FACE)
- 🎨 **Diseño Moderno**: UI responsive y atractiva
- ⚡ **Sin dependencias**: HTML/CSS/JS vanilla, sin frameworks

## Estructura

```
frontend/
├── index.html              # Página principal (SPA)
├── static/
│   ├── css/
│   │   └── styles.css      # Estilos CSS
│   └── js/
│       └── app.js          # Lógica de la aplicación
└── README.md
```

## Instalación y Uso

### Opción 1: Servidor Python Simple

```bash
cd frontend
python -m http.server 8000
```

Abre en el navegador: `http://localhost:8000`

### Opción 2: Servir con Nginx

Ya está configurado en `nginx/nginx.conf` para servir el frontend desde `/app/frontend`

### Opción 3: Live Server (VS Code)

Instala la extensión "Live Server" y haz clic derecho en `index.html` → "Open with Live Server"

## Configuración

### API URL

El frontend se conecta al backend en `http://localhost:5000/api/v1` por defecto.

Para cambiar la URL del backend, edita en `static/js/app.js`:

```javascript
const API_URL = 'http://tu-servidor:puerto/api/v1';
```

### CORS

Asegúrate de que el backend tenga CORS configurado para permitir requests desde el frontend:

```python
# En backend/app/main.py
app.add_middleware(
    CORSMiddleware,
    allow_origins=["http://localhost:8000", "http://localhost:3000"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)
```

## Uso

### 1. Registro de Usuario

1. Haz clic en "Login" en la navegación
2. Haz clic en "Regístrate"
3. Completa el formulario:
   - Nombre de usuario
   - Email
   - Contraseña (mínimo 8 caracteres)
4. Haz clic en "Crear Cuenta"

### 2. Inicio de Sesión

1. Ingresa tu email y contraseña
2. Haz clic en "Entrar"
3. Serás redirigido al dashboard de trabajos

### 3. Subir Proteína

1. Haz clic en "Subir" en la navegación
2. Completa el formulario:
   - Nombre de la proteína (ej: `1AHW_l_u`)
   - Archivo STL
   - Archivo Vertices (.vert)
   - Archivo Faces (.face)
   - Tipo de procesamiento:
     - **Parte 1**: Context Rays (15-35 min)
     - **Parte 2**: Layer Evaluation + Unity (10-20 min)
3. Haz clic en "Procesar"
4. El trabajo se creará y empezará a procesarse

### 4. Ver Trabajos

1. Haz clic en "Trabajos" en la navegación
2. Verás todos tus trabajos con:
   - Estado (Pendiente, Procesando, Completado, Fallido)
   - Progreso (0-100%)
   - Fecha de creación
   - Información de la proteína

Los trabajos en proceso se actualizan automáticamente cada 5 segundos.

## Páginas

### Home

Página de bienvenida con:
- Descripción de la plataforma
- Características principales
- Botón para comenzar

### Login/Register

Formularios de autenticación:
- Validación de campos
- Mensajes de error
- Cambio entre login y registro

### Jobs Dashboard

Lista de trabajos con:
- Tarjetas visuales por cada trabajo
- Estados con colores distintivos
- Barra de progreso animada
- Auto-actualización para trabajos en proceso

### Upload

Formulario de subida con:
- Campos para los 3 archivos requeridos
- Selección de tipo de procesamiento
- Validación de archivos
- Feedback de progreso

## Características Técnicas

### Autenticación

- JWT tokens almacenados en localStorage
- Access token + Refresh token
- Refresh automático cuando expira el access token
- Logout limpia todos los tokens

### Gestión de Estado

- Estado global simple con variables JavaScript
- LocalStorage para persistencia
- UI reactiva a cambios de autenticación

### Comunicación con API

- Fetch API para requests HTTP
- Manejo de errores con try/catch
- Headers de autorización automáticos
- FormData para upload de archivos

### UI/UX

- Single Page Application (SPA)
- Navegación sin recargar página
- Animaciones suaves (CSS transitions)
- Toast notifications para feedback
- Diseño responsive (mobile-friendly)
- Loading states y feedback visual

## Estilos

### Variables CSS

El diseño usa variables CSS para fácil personalización:

```css
:root {
    --primary: #2563eb;        /* Color principal */
    --secondary: #10b981;      /* Color secundario */
    --danger: #ef4444;         /* Error/peligro */
    --success: #22c55e;        /* Éxito */
    /* ... más variables */
}
```

Para cambiar los colores, edita estas variables en `static/css/styles.css`.

### Responsive Design

Breakpoints:
- Mobile: < 768px
- Tablet: 768px - 1024px
- Desktop: > 1024px

## Limitaciones Actuales

1. **No hay WebSocket**: Los trabajos se actualizan por polling (cada 5 segundos)
2. **No hay descarga de resultados**: Solo muestra estado
3. **No hay visualización de proteínas**: Solo gestión de trabajos
4. **No hay paginación**: Todos los trabajos se cargan a la vez
5. **No hay filtros/búsqueda**: Lista simple de trabajos

## Mejoras Futuras

### Corto Plazo
- [ ] Integrar WebSocket para updates en tiempo real
- [ ] Botón de descarga de resultados
- [ ] Paginación de trabajos
- [ ] Filtros por estado/fecha

### Medio Plazo
- [ ] Visualización 3D de proteínas (Three.js)
- [ ] Gráficos de progreso (Chart.js)
- [ ] Admin panel para usuarios admin
- [ ] Profile page para editar datos

### Largo Plazo
- [ ] Compartir resultados con otros usuarios
- [ ] Comentarios y anotaciones en proteínas
- [ ] Comparación de resultados
- [ ] Exportar reportes en PDF

## Desarrollo

### Agregar Nueva Página

1. Añade un div con id `nombrePage` en `index.html`:
```html
<div id="miNuevaPaginaPage" class="page">
    <!-- Contenido aquí -->
</div>
```

2. Añade link de navegación:
```html
<a href="#" data-page="miNuevaPagina">Mi Página</a>
```

3. La función `showPage()` manejará la navegación automáticamente

### Agregar Nuevo Endpoint

En `app.js`, crea una nueva función:

```javascript
async function miNuevaFuncion() {
    try {
        const response = await fetch(`${API_URL}/mi-endpoint`, {
            headers: {
                'Authorization': `Bearer ${authToken}`
            }
        });

        const data = await response.json();
        // Procesar datos...
    } catch (error) {
        console.error('Error:', error);
        showToast('Error al procesar', 'error');
    }
}
```

## Troubleshooting

### CORS Errors

Si ves errores de CORS en la consola:
1. Verifica que el backend tenga CORS habilitado
2. Verifica que la URL del frontend esté en `allow_origins`
3. Usa un servidor (no abras el archivo HTML directamente)

### 401 Unauthorized

Si todos los requests fallan con 401:
1. Verifica que el token se esté guardando en localStorage
2. Verifica que el backend esté corriendo
3. Intenta hacer logout y login de nuevo

### Archivos No se Suben

Si el upload falla:
1. Verifica el tamaño de los archivos (< 100MB)
2. Verifica las extensiones (.stl, .vert, .face)
3. Revisa la consola del navegador para errores
4. Verifica que el backend tenga permisos de escritura

## Soporte

Para reportar bugs o solicitar features:
- Abre un issue en GitHub
- Contacta al equipo de desarrollo

---

**Versión**: 1.0.0
**Última actualización**: 2025-11-13
