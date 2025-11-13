# Protein Docking Platform - Frontend

Frontend moderno de **producción** construido con React + TypeScript + Vite.

## 🚀 Stack Tecnológico

### Core
- **React 18** - UI library con hooks
- **TypeScript 5** - Type safety
- **Vite 5** - Build tool ultra-rápido
- **React Router 6** - Client-side routing

### State Management & Data Fetching
- **Zustand** - Estado global simple y performante
- **TanStack Query (React Query)** - Data fetching, caching, synchronization
- **Axios** - HTTP client con interceptors

### Real-time
- **Socket.IO Client** - WebSocket para updates en tiempo real

### Styling
- **Tailwind CSS** - Utility-first CSS framework
- **Lucide React** - Iconos modernos
- **clsx** - Class name utilities

### DevTools
- **ESLint** - Code linting
- **Prettier** - Code formatting
- **TypeScript** - Type checking

## 📁 Estructura del Proyecto

```
frontend/
├── src/
│   ├── components/          # Componentes reutilizables (a completar)
│   ├── pages/              # Páginas de la aplicación (a completar)
│   ├── hooks/              # Custom React hooks
│   │   ├── useAuth.ts      # Hook de autenticación
│   │   ├── useJobs.ts      # Hook de trabajos
│   │   ├── useProteins.ts  # Hook de proteínas
│   │   └── useSocket.ts    # Hook de WebSocket
│   ├── services/           # API & servicios externos
│   │   ├── api.ts          # Cliente Axios con interceptors
│   │   └── socket.ts       # Cliente Socket.IO
│   ├── store/              # Estado global
│   │   └── authStore.ts    # Store de autenticación (Zustand)
│   ├── types/              # TypeScript types
│   │   └── index.ts        # Todos los tipos e interfaces
│   ├── utils/              # Utilidades
│   │   ├── toast.ts        # Sistema de notificaciones
│   │   └── format.ts       # Formateo de fechas/números
│   ├── App.tsx             # Componente raíz (a crear)
│   ├── main.tsx            # Entry point (a crear)
│   └── index.css           # Estilos globales
├── package.json            # Dependencias y scripts
├── tsconfig.json           # Configuración TypeScript
├── vite.config.ts          # Configuración Vite
├── tailwind.config.js      # Configuración Tailwind
└── postcss.config.js       # Configuración PostCSS
```

## 🎯 Características Implementadas

### ✅ Infraestructura Base (100%)
- ✅ Configuración completa de TypeScript
- ✅ Setup de Vite con HMR
- ✅ Tailwind CSS configurado
- ✅ Path aliases (`@/*`)
- ✅ ESLint + Prettier

### ✅ API Integration (100%)
- ✅ Cliente Axios con interceptors
- ✅ Auto-refresh de tokens JWT
- ✅ Manejo de errores global
- ✅ Retry logic en 401
- ✅ Type-safe API calls

### ✅ State Management (100%)
- ✅ Zustand store para auth
- ✅ Persistencia en localStorage
- ✅ React Query para server state
- ✅ Auto-refetch cada 5 segundos
- ✅ Optimistic updates

### ✅ Real-time Updates (100%)
- ✅ Socket.IO client configurado
- ✅ Auto-reconnection
- ✅ Job updates en tiempo real
- ✅ Toast notifications
- ✅ Query invalidation automática

### ✅ Custom Hooks (100%)
- ✅ `useAuth` - Login, register, logout
- ✅ `useJobs` - CRUD de trabajos
- ✅ `useProteins` - CRUD de proteínas
- ✅ `useSocket` - WebSocket integration

### ✅ Utilities (100%)
- ✅ Toast system con animaciones
- ✅ Formato de fechas (date-fns)
- ✅ Formato de duración/tamaño
- ✅ Error handling helpers

### ⏳ UI Components (0% - Por Implementar)
Los componentes y páginas necesitan ser creados:
- [ ] Layout components (Header, Sidebar, Footer)
- [ ] Form components (Input, Button, Select, FileUpload)
- [ ] UI components (Card, Badge, Progress, Modal)
- [ ] Pages (Login, Register, Dashboard, Upload)

## 🔧 Instalación

### Requisitos
- Node.js 18+ (recomendado 20+)
- npm 9+ o pnpm 8+

### Pasos

1. **Instalar dependencias**
```bash
cd frontend
npm install
```

2. **Configurar variables de entorno**
```bash
cp .env.example .env
```

Editar `.env`:
```env
VITE_API_URL=http://localhost:5000/api/v1
VITE_SOCKET_URL=http://localhost:8080
```

3. **Iniciar desarrollo**
```bash
npm run dev
```

La app estará disponible en `http://localhost:3000`

## 📝 Scripts Disponibles

```bash
# Desarrollo
npm run dev          # Inicia servidor de desarrollo (HMR)

# Build
npm run build        # Build para producción
npm run preview      # Preview del build

# Code Quality
npm run lint         # Ejecuta ESLint
npm run format       # Formatea código con Prettier
```

## 🏗️ Próximos Pasos de Implementación

Para completar el frontend, necesitas crear:

### 1. Componentes UI Base
```
src/components/ui/
├── Button.tsx
├── Input.tsx
├── Card.tsx
├── Badge.tsx
├── Progress.tsx
├── Modal.tsx
└── FileUpload.tsx
```

### 2. Layout Components
```
src/components/layout/
├── Header.tsx
├── Sidebar.tsx
└── MainLayout.tsx
```

### 3. Feature Components
```
src/components/
├── JobCard.tsx
├── JobList.tsx
├── ProteinCard.tsx
├── UploadForm.tsx
└── LoginForm.tsx
```

### 4. Pages
```
src/pages/
├── LandingPage.tsx
├── LoginPage.tsx
├── RegisterPage.tsx
├── DashboardPage.tsx
└── UploadPage.tsx
```

### 5. App Setup
```tsx
// src/main.tsx
import React from 'react'
import ReactDOM from 'react-dom/client'
import { BrowserRouter } from 'react-router-dom'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import App from './App'
import './index.css'

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 1000 * 60, // 1 minute
      refetchOnWindowFocus: false,
    },
  },
})

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <BrowserRouter>
      <QueryClientProvider client={queryClient}>
        <App />
      </QueryClientProvider>
    </BrowserRouter>
  </React.StrictMode>
)
```

```tsx
// src/App.tsx
import { Routes, Route, Navigate } from 'react-router-dom'
import { useAuthStore } from '@/store/authStore'
import { useSocket } from '@/hooks/useSocket'

// Import your pages here
// import LandingPage from '@/pages/LandingPage'
// import LoginPage from '@/pages/LoginPage'
// etc...

function App() {
  const isAuthenticated = useAuthStore((state) => state.isAuthenticated)
  useSocket() // Initialize WebSocket

  return (
    <Routes>
      <Route path="/" element={<div>Landing Page</div>} />
      <Route path="/login" element={<div>Login Page</div>} />
      <Route path="/register" element={<div>Register Page</div>} />
      <Route
        path="/dashboard"
        element={isAuthenticated ? <div>Dashboard</div> : <Navigate to="/login" />}
      />
      <Route
        path="/upload"
        element={isAuthenticated ? <div>Upload Page</div> : <Navigate to="/login" />}
      />
    </Routes>
  )
}

export default App
```

## 🎨 Guía de Estilo

### Tailwind Classes Personalizadas

Ya hay configuradas en `tailwind.config.js`:

```js
// Colores primarios
className="bg-primary-500 text-white"
className="bg-primary-600 hover:bg-primary-700"

// Animaciones
className="animate-fade-in"
className="animate-slide-in"
```

### Ejemplo de Componente

```tsx
import { useState } from 'react'
import { useLogin } from '@/hooks/useAuth'

export function LoginForm() {
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const login = useLogin()

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    login.mutate({ username: email, password })
  }

  return (
    <form onSubmit={handleSubmit} className="space-y-4">
      <input
        type="email"
        value={email}
        onChange={(e) => setEmail(e.target.value)}
        className="w-full px-4 py-2 border rounded-lg"
        placeholder="Email"
      />
      <input
        type="password"
        value={password}
        onChange={(e) => setPassword(e.target.value)}
        className="w-full px-4 py-2 border rounded-lg"
        placeholder="Password"
      />
      <button
        type="submit"
        disabled={login.isPending}
        className="w-full px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 disabled:opacity-50"
      >
        {login.isPending ? 'Loading...' : 'Login'}
      </button>
    </form>
  )
}
```

## 🔌 Integración con Backend

El frontend está configurado para conectarse al backend a través de:

### REST API
- Base URL: `http://localhost:5000/api/v1`
- Proxy configurado en Vite para desarrollo
- Auto-refresh de JWT tokens
- Type-safe con TypeScript

### WebSocket
- Socket.IO en: `http://localhost:8080`
- Autenticación con JWT
- Auto-reconnection
- Job updates en tiempo real

## 🧪 Testing (Próximo Paso)

Para agregar tests:

```bash
npm install -D vitest @testing-library/react @testing-library/jest-dom
```

```tsx
// Example test
import { render, screen } from '@testing-library/react'
import { LoginForm } from './LoginForm'

test('renders login form', () => {
  render(<LoginForm />)
  expect(screen.getByPlaceholderText('Email')).toBeInTheDocument()
})
```

## 📦 Build para Producción

```bash
# Build
npm run build

# Output en ./dist
# Archivos estáticos listos para deploy

# Preview local
npm run preview
```

### Deployment

El build genera archivos estáticos en `dist/` que pueden ser servidos por:
- Nginx
- Vercel
- Netlify
- Cloudflare Pages
- AWS S3 + CloudFront

## 🎯 Ventajas de esta Arquitectura

### Type Safety
- **100% TypeScript**: Todos los tipos definidos
- Autocompletado en IDE
- Catch errors en compile-time

### Performance
- **Vite**: Build instantáneo con HMR
- **React Query**: Smart caching y deduplication
- **Code splitting**: Automatic con Vite
- **Tree shaking**: Bundle size optimizado

### Developer Experience
- Hot Module Replacement (HMR)
- Path aliases (`@/*`)
- ESLint + Prettier configurados
- TypeScript errors en el IDE

### Scalability
- Estructura modular
- Separation of concerns
- Custom hooks reutilizables
- Fácil de extender

### Real-time
- WebSocket con Socket.IO
- Auto-invalidation de queries
- Notificaciones instantáneas
- Optimistic updates

## 🐛 Troubleshooting

### Error: Cannot find module '@/*'
```bash
# Reiniciar TypeScript server en VS Code
Cmd/Ctrl + Shift + P -> TypeScript: Restart TS Server
```

### Puerto 3000 ya en uso
```bash
# Cambiar puerto en vite.config.ts
server: {
  port: 3001
}
```

### CORS errors
- Verificar que el backend tenga CORS habilitado
- Verificar proxy en `vite.config.ts`

## 📚 Recursos

- [React Docs](https://react.dev/)
- [TypeScript Handbook](https://www.typescriptlang.org/docs/)
- [TanStack Query](https://tanstack.com/query/latest)
- [Zustand](https://github.com/pmndrs/zustand)
- [Tailwind CSS](https://tailwindcss.com/docs)
- [Vite](https://vitejs.dev/)

---

**Version**: 2.0.0
**Stack**: React 18 + TypeScript 5 + Vite 5
**Status**: Base Infrastructure Complete (UI to be implemented)
