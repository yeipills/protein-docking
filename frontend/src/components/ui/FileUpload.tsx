import { useRef, ChangeEvent } from 'react'
import { clsx } from 'clsx'
import { Upload, X, File } from 'lucide-react'
import { formatFileSize } from '@/utils/format'

interface FileUploadProps {
  label?: string
  accept?: string
  error?: string
  value?: File | null
  onChange: (file: File | null) => void
  helperText?: string
}

export function FileUpload({
  label,
  accept,
  error,
  value,
  onChange,
  helperText,
}: FileUploadProps) {
  const inputRef = useRef<HTMLInputElement>(null)

  const handleChange = (e: ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0] || null
    onChange(file)
  }

  const handleClear = () => {
    onChange(null)
    if (inputRef.current) {
      inputRef.current.value = ''
    }
  }

  return (
    <div className="w-full">
      {label && <label className="block text-sm font-medium text-gray-700 mb-1">{label}</label>}

      <div
        className={clsx(
          'relative border-2 border-dashed rounded-lg p-6 transition-colors',
          error ? 'border-red-500' : 'border-gray-300 hover:border-primary-400',
          value ? 'bg-gray-50' : 'bg-white'
        )}
      >
        {!value ? (
          <div
            className="flex flex-col items-center justify-center cursor-pointer"
            onClick={() => inputRef.current?.click()}
          >
            <Upload className="h-10 w-10 text-gray-400 mb-2" />
            <p className="text-sm text-gray-600 mb-1">
              Haz clic para seleccionar un archivo
            </p>
            {helperText && <p className="text-xs text-gray-500">{helperText}</p>}
          </div>
        ) : (
          <div className="flex items-center justify-between">
            <div className="flex items-center space-x-3">
              <File className="h-8 w-8 text-primary-600" />
              <div>
                <p className="text-sm font-medium text-gray-900">{value.name}</p>
                <p className="text-xs text-gray-500">{formatFileSize(value.size)}</p>
              </div>
            </div>
            <button
              type="button"
              onClick={handleClear}
              className="p-1 rounded-full hover:bg-gray-200 transition-colors"
            >
              <X className="h-5 w-5 text-gray-500" />
            </button>
          </div>
        )}

        <input
          ref={inputRef}
          type="file"
          accept={accept}
          onChange={handleChange}
          className="hidden"
        />
      </div>

      {error && <p className="mt-1 text-sm text-red-600">{error}</p>}
    </div>
  )
}
