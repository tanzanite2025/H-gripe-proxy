import { Button } from '@/components/tailwind'
import { useLockFn } from 'ahooks'
import { useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

interface Props {
  onChange: (file: File, value: string) => void
}

export const FileInput = (props: Props) => {
  const { onChange } = props

  const { t } = useTranslation()
  const inputRef = useRef<any>(undefined)
  const [loading, setLoading] = useState(false)
  const [fileName, setFileName] = useState('')

  const onFileInput = useLockFn(async (e: any) => {
    const file = e.target.files?.[0] as File

    if (!file) return

    setFileName(file.name)
    setLoading(true)

    return new Promise((resolve, reject) => {
      const reader = new FileReader()
      reader.onload = (event) => {
        resolve(null)
        onChange(file, event.target?.result as string)
      }
      reader.onerror = reject
      reader.readAsText(file)
    }).finally(() => setLoading(false))
  })

  return (
    <div className="mb-1 mt-2 flex items-center">
      <Button
        variant="outlined"
        className="flex-none"
        onClick={() => inputRef.current?.click()}
      >
        {t('profiles.components.fileInput.chooseFile')}
      </Button>

      <input
        type="file"
        accept=".yaml,.yml"
        ref={inputRef}
        style={{ display: 'none' }}
        onChange={onFileInput}
      />

      <span className="ml-2 overflow-hidden text-ellipsis whitespace-nowrap">
        {loading ? 'Loading...' : fileName}
      </span>
    </div>
  )
}
