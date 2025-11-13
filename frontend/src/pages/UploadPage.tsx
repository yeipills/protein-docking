import { MainLayout } from '@/components/layout/MainLayout'
import { UploadForm } from '@/components/UploadForm'

export function UploadPage() {
  return (
    <MainLayout>
      <div className="max-w-3xl mx-auto">
        <div className="mb-8">
          <h1 className="text-3xl font-bold text-gray-900">
            Subir Nueva Proteína
          </h1>
          <p className="text-gray-600 mt-2">
            Sube los archivos de tu proteína para iniciar el procesamiento
          </p>
        </div>

        <UploadForm />
      </div>
    </MainLayout>
  )
}
