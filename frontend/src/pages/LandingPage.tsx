import { useNavigate } from 'react-router-dom'
import { Button } from '@/components/ui/Button'
import { Header } from '@/components/layout/Header'
import { Zap, Lock, TrendingUp, Dna } from 'lucide-react'

export function LandingPage() {
  const navigate = useNavigate()

  return (
    <div className="min-h-screen bg-gradient-to-br from-primary-600 via-purple-600 to-pink-600">
      <Header />

      {/* Hero Section */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-20">
        <div className="text-center">
          <Dna className="h-20 w-20 text-white mx-auto mb-8 animate-pulse" />
          <h1 className="text-5xl font-bold text-white mb-6 leading-tight">
            Plataforma de Protein Docking
          </h1>
          <p className="text-xl text-white/90 mb-12 max-w-2xl mx-auto">
            Procesamiento automatizado de proteínas con algoritmos científicos de última generación.
            Rápido, seguro y escalable.
          </p>

          <div className="flex justify-center gap-4">
            <Button
              size="lg"
              variant="primary"
              onClick={() => navigate('/register')}
              className="bg-white text-primary-600 hover:bg-gray-100"
            >
              Comenzar Ahora
            </Button>
            <Button
              size="lg"
              variant="ghost"
              onClick={() => navigate('/login')}
              className="text-white border-2 border-white hover:bg-white/10"
            >
              Iniciar Sesión
            </Button>
          </div>
        </div>

        {/* Features Grid */}
        <div className="grid md:grid-cols-3 gap-8 mt-20">
          <FeatureCard
            icon={<Zap className="h-10 w-10" />}
            title="Rápido"
            description="Pipeline completo en 25-55 minutos. Optimizado con Cython para máximo rendimiento."
          />
          <FeatureCard
            icon={<Lock className="h-10 w-10" />}
            title="Seguro"
            description="Autenticación JWT, encriptación de datos y control de acceso basado en roles."
          />
          <FeatureCard
            icon={<TrendingUp className="h-10 w-10" />}
            title="Escalable"
            description="Soporta 100-1000+ usuarios concurrentes con arquitectura de microservicios."
          />
        </div>

        {/* Stats */}
        <div className="mt-20 grid md:grid-cols-4 gap-8 text-center">
          <StatCard number="5" label="Algoritmos Científicos" />
          <StatCard number="100%" label="Cobertura de Tests" />
          <StatCard number="25-55" label="Minutos por Proteína" />
          <StatCard number="1000+" label="Usuarios Soportados" />
        </div>
      </div>
    </div>
  )
}

interface FeatureCardProps {
  icon: React.ReactNode
  title: string
  description: string
}

function FeatureCard({ icon, title, description }: FeatureCardProps) {
  return (
    <div className="bg-white/10 backdrop-blur-sm rounded-xl p-6 text-white hover:bg-white/20 transition-all">
      <div className="text-white mb-4">{icon}</div>
      <h3 className="text-xl font-bold mb-2">{title}</h3>
      <p className="text-white/80">{description}</p>
    </div>
  )
}

interface StatCardProps {
  number: string
  label: string
}

function StatCard({ number, label }: StatCardProps) {
  return (
    <div className="bg-white/10 backdrop-blur-sm rounded-xl p-6 text-white">
      <div className="text-4xl font-bold mb-2">{number}</div>
      <div className="text-white/80">{label}</div>
    </div>
  )
}
