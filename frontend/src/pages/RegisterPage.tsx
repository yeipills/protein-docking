import { useState, FormEvent } from 'react'
import { Link } from 'react-router-dom'
import { MainLayout } from '@/components/layout/MainLayout'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/Card'
import { Input } from '@/components/ui/Input'
import { Button } from '@/components/ui/Button'
import { useRegister } from '@/hooks/useAuth'

export function RegisterPage() {
  const register = useRegister()
  const [username, setUsername] = useState('')
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [confirmPassword, setConfirmPassword] = useState('')
  const [errors, setErrors] = useState<Record<string, string>>({})

  const validate = (): boolean => {
    const newErrors: Record<string, string> = {}

    if (!username.trim()) {
      newErrors.username = 'El nombre de usuario es requerido'
    } else if (username.length < 3) {
      newErrors.username = 'Mínimo 3 caracteres'
    }

    if (!email.trim()) {
      newErrors.email = 'El email es requerido'
    } else if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email)) {
      newErrors.email = 'Email inválido'
    }

    if (!password) {
      newErrors.password = 'La contraseña es requerida'
    } else if (password.length < 12) {
      newErrors.password = 'Mínimo 12 caracteres'
    } else if (password.length > 72) {
      newErrors.password = 'Máximo 72 caracteres'
    } else if (!/[A-Z]/.test(password)) {
      newErrors.password = 'Debe contener al menos una mayúscula'
    } else if (!/[a-z]/.test(password)) {
      newErrors.password = 'Debe contener al menos una minúscula'
    } else if (!/\d/.test(password)) {
      newErrors.password = 'Debe contener al menos un número'
    } else if (!/[!@#$%^&*(),.?":{}|<>]/.test(password)) {
      newErrors.password = 'Debe contener al menos un carácter especial (!@#$%^&*)'
    }

    if (password !== confirmPassword) {
      newErrors.confirmPassword = 'Las contraseñas no coinciden'
    }

    setErrors(newErrors)
    return Object.keys(newErrors).length === 0
  }

  const handleSubmit = (e: FormEvent) => {
    e.preventDefault()

    if (!validate()) {
      return
    }

    register.mutate({ username, email, password })
  }

  return (
    <MainLayout>
      <div className="max-w-md mx-auto mt-12">
        <Card>
          <CardHeader>
            <CardTitle className="text-2xl text-center">Crear Cuenta</CardTitle>
          </CardHeader>
          <CardContent>
            <form onSubmit={handleSubmit} className="space-y-4">
              <Input
                label="Nombre de Usuario"
                type="text"
                placeholder="usuario123"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                error={errors.username}
                autoComplete="username"
                disabled={register.isPending}
              />

              <Input
                label="Email"
                type="email"
                placeholder="tu@email.com"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                error={errors.email}
                autoComplete="email"
                disabled={register.isPending}
              />

              <Input
                label="Contraseña"
                type="password"
                placeholder="••••••••"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                error={errors.password}
                helperText="12-72 caracteres, con mayúscula, minúscula, número y carácter especial"
                autoComplete="new-password"
                disabled={register.isPending}
              />

              <Input
                label="Confirmar Contraseña"
                type="password"
                placeholder="••••••••"
                value={confirmPassword}
                onChange={(e) => setConfirmPassword(e.target.value)}
                error={errors.confirmPassword}
                autoComplete="new-password"
                disabled={register.isPending}
              />

              <Button
                type="submit"
                variant="primary"
                className="w-full"
                isLoading={register.isPending}
              >
                {register.isPending ? 'Creando cuenta...' : 'Crear Cuenta'}
              </Button>
            </form>

            <p className="text-center text-sm text-gray-600 mt-6">
              ¿Ya tienes cuenta?{' '}
              <Link to="/login" className="text-primary-600 hover:text-primary-700 font-medium">
                Inicia sesión
              </Link>
            </p>
          </CardContent>
        </Card>
      </div>
    </MainLayout>
  )
}
