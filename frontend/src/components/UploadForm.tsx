import { useState, FormEvent } from 'react'
import { useNavigate } from 'react-router-dom'
import { Card, CardContent, CardHeader, CardTitle } from './ui/Card'
import { Input } from './ui/Input'
import { FileUpload } from './ui/FileUpload'
import { Button } from './ui/Button'
import { useUploadProtein } from '@/hooks/useProteins'
import { useCreateJob } from '@/hooks/useJobs'
import { JobType } from '@/types'

export function UploadForm() {
  const navigate = useNavigate()
  const uploadProtein = useUploadProtein()
  const createJob = useCreateJob()

  const [proteinName, setProteinName] = useState('')
  const [stlFile, setStlFile] = useState<File | null>(null)
  const [vertFile, setVertFile] = useState<File | null>(null)
  const [faceFile, setFaceFile] = useState<File | null>(null)
  const [jobType, setJobType] = useState<JobType>(JobType.PART_ONE)

  const [errors, setErrors] = useState<Record<string, string>>({})

  const validate = (): boolean => {
    const newErrors: Record<string, string> = {}

    if (!proteinName.trim()) {
      newErrors.proteinName = 'El nombre es requerido'
    }

    if (!stlFile) {
      newErrors.stlFile = 'El archivo STL es requerido'
    } else if (!stlFile.name.endsWith('.stl')) {
      newErrors.stlFile = 'Debe ser un archivo .stl'
    }

    if (!vertFile) {
      newErrors.vertFile = 'El archivo VERT es requerido'
    } else if (!vertFile.name.endsWith('.vert')) {
      newErrors.vertFile = 'Debe ser un archivo .vert'
    }

    if (!faceFile) {
      newErrors.faceFile = 'El archivo FACE es requerido'
    } else if (!faceFile.name.endsWith('.face')) {
      newErrors.faceFile = 'Debe ser un archivo .face'
    }

    setErrors(newErrors)
    return Object.keys(newErrors).length === 0
  }

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault()

    if (!validate()) {
      return
    }

    try {
      // First, upload protein files
      const protein = await uploadProtein.mutateAsync({
        name: proteinName,
        stlFile: stlFile!,
        verticesFile: vertFile!,
        facesFile: faceFile!,
      })

      // Then create job
      await createJob.mutateAsync({
        protein_id: protein.id,
        job_type: jobType,
      })

      // Navigate to dashboard
      navigate('/dashboard')
    } catch (error) {
      // Error handling is done in the hooks
      console.error('Upload failed:', error)
    }
  }

  const isLoading = uploadProtein.isPending || createJob.isPending

  return (
    <Card>
      <CardHeader>
        <CardTitle>Subir Proteína</CardTitle>
      </CardHeader>
      <CardContent>
        <form onSubmit={handleSubmit} className="space-y-6">
          {/* Protein Name */}
          <Input
            label="Nombre de la Proteína"
            placeholder="ej: 1AHW_l_u"
            value={proteinName}
            onChange={(e) => setProteinName(e.target.value)}
            error={errors.proteinName}
            disabled={isLoading}
          />

          {/* File Uploads */}
          <div className="space-y-4">
            <FileUpload
              label="Archivo STL"
              accept=".stl"
              value={stlFile}
              onChange={setStlFile}
              error={errors.stlFile}
              helperText="Archivo de superficie 3D de la proteína"
            />

            <FileUpload
              label="Archivo Vertices (.vert)"
              accept=".vert"
              value={vertFile}
              onChange={setVertFile}
              error={errors.vertFile}
              helperText="Archivo MSMS de vértices"
            />

            <FileUpload
              label="Archivo Faces (.face)"
              accept=".face"
              value={faceFile}
              onChange={setFaceFile}
              error={errors.faceFile}
              helperText="Archivo MSMS de caras"
            />
          </div>

          {/* Job Type */}
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              Tipo de Procesamiento
            </label>
            <div className="space-y-2">
              <label className="flex items-center space-x-3 p-3 border rounded-lg cursor-pointer hover:bg-gray-50 transition-colors">
                <input
                  type="radio"
                  name="jobType"
                  value={JobType.PART_ONE}
                  checked={jobType === JobType.PART_ONE}
                  onChange={(e) => setJobType(e.target.value as JobType)}
                  disabled={isLoading}
                  className="text-primary-600 focus:ring-primary-500"
                />
                <div>
                  <p className="font-medium text-gray-900">Parte 1: Context Rays</p>
                  <p className="text-sm text-gray-500">Generación de rayos de contexto (15-35 min)</p>
                </div>
              </label>

              <label className="flex items-center space-x-3 p-3 border rounded-lg cursor-pointer hover:bg-gray-50 transition-colors">
                <input
                  type="radio"
                  name="jobType"
                  value={JobType.PART_TWO}
                  checked={jobType === JobType.PART_TWO}
                  onChange={(e) => setJobType(e.target.value as JobType)}
                  disabled={isLoading}
                  className="text-primary-600 focus:ring-primary-500"
                />
                <div>
                  <p className="font-medium text-gray-900">Parte 2: Layer Evaluation</p>
                  <p className="text-sm text-gray-500">Evaluación de capas + Export Unity (10-20 min)</p>
                </div>
              </label>
            </div>
          </div>

          {/* Submit Button */}
          <div className="flex justify-end space-x-3">
            <Button
              type="button"
              variant="ghost"
              onClick={() => navigate('/dashboard')}
              disabled={isLoading}
            >
              Cancelar
            </Button>
            <Button
              type="submit"
              variant="primary"
              isLoading={isLoading}
            >
              {isLoading ? 'Subiendo...' : 'Procesar Proteína'}
            </Button>
          </div>
        </form>
      </CardContent>
    </Card>
  )
}
