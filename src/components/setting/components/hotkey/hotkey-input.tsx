import { Trash2 } from 'lucide-react'
import { useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { Box, IconButton } from '@/components/tailwind'
import { parseHotkey } from '@/utils/format'

interface Props {
  value: string[]
  onChange: (value: string[]) => void
}

export const HotkeyInput = (props: Props) => {
  const { value, onChange } = props
  const { t } = useTranslation()

  const changeRef = useRef<string[]>([])
  const [keys, setKeys] = useState(value)

  return (
    <Box className="flex items-center">
      <div className="relative w-[230px] min-h-[36px]">
        <input
          className="absolute top-0 left-0 w-full h-full z-10 opacity-0"
          onKeyUp={() => {
            const ret = changeRef.current.slice()
            if (ret.length) {
              onChange(ret)
              changeRef.current = []
            }
          }}
          onKeyDown={(e) => {
            e.preventDefault()
            e.stopPropagation()

            const key = parseHotkey(e)
            if (key === 'UNIDENTIFIED') return

            changeRef.current = [...new Set([...changeRef.current, key])]
            setKeys(changeRef.current)
          }}
        />

        <div className="flex items-center flex-wrap w-full h-full min-h-[36px] box-border p-1 border border-divider rounded focus-within:border-primary/75">
          {keys.map((key, index) => (
            <Box className="flex" key={key}>
              <span className="leading-[25px] px-0.5" hidden={index === 0}>
                +
              </span>
              <div className="text-sm text-text-primary border border-divider/20 rounded-sm px-[5px] py-0.5 my-0.5">
                {key}
              </div>
            </Box>
          ))}
        </div>
      </div>

      <IconButton
        size="small"
        title={t('shared.actions.delete')}
        onClick={() => {
          onChange([])
          setKeys([])
        }}
      >
        <Trash2 size={16} />
      </IconButton>
    </Box>
  )
}
