import { Trash2 } from 'lucide-react'
import type { ChangeEvent } from 'react'

import { Button, TextField } from '@/components/tailwind'

interface SelfDestructPanelProps {
  selfDestructConfirm: string
  onSelfDestructConfirmChange: (value: string) => void
  onSelfDestruct: () => void
}

export default function SelfDestructPanel({
  selfDestructConfirm,
  onSelfDestructConfirmChange,
  onSelfDestruct,
}: SelfDestructPanelProps) {
  return (
    <div className="p-4 bg-card border-2 border-red-500 rounded-lg">
      <h3 className="text-sm font-semibold mb-4 text-red-500">
        紧急自毁机制
      </h3>
      <div className="space-y-4">
        <p className="text-sm text-muted-foreground">
          检测到安全威胁时，自动清除内存中的密钥、擦除本地缓存并退出程序
        </p>

        <TextField
          label="确认码"
          value={selfDestructConfirm}
          onChange={(event: ChangeEvent<HTMLInputElement>) =>
            onSelfDestructConfirmChange(event.target.value)
          }
          placeholder="输入 CONFIRM_SELF_DESTRUCT"
          fullWidth
          helperText="手动触发自毁需要输入确认码"
        />

        <Button
          variant="destructive"
          onClick={onSelfDestruct}
          disabled={selfDestructConfirm !== 'CONFIRM_SELF_DESTRUCT'}
          className="w-full"
        >
          <Trash2 className="w-4 h-4 mr-2" />
          触发自毁
        </Button>
      </div>
    </div>
  )
}
