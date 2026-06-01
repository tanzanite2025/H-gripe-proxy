import type { ChangeEvent } from 'react'

import { Button, TextField } from '@/components/tailwind'

interface HoneypotDecoyPanelProps {
  decoyPath: string
  onDecoyPathChange: (path: string) => void
  onDeployDecoy: () => void
  onCleanupDecoy: () => void
  onCheckDecoyAccess: () => void
}

export default function HoneypotDecoyPanel({
  decoyPath,
  onDecoyPathChange,
  onDeployDecoy,
  onCleanupDecoy,
  onCheckDecoyAccess,
}: HoneypotDecoyPanelProps) {
  return (
    <div className="p-4 bg-card border border-border rounded-lg">
      <h3 className="text-sm font-semibold mb-4">配置文件欺骗</h3>
      <div className="space-y-4">
        <TextField
          label="假配置文件路径"
          value={decoyPath}
          onChange={(event: ChangeEvent<HTMLInputElement>) =>
            onDecoyPathChange(event.target.value)
          }
          fullWidth
          helperText="放置假配置文件来误导扫描软件"
        />
        <div className="flex gap-2">
          <Button variant="default" onClick={onDeployDecoy}>
            部署假配置
          </Button>
          <Button variant="outline" onClick={onCheckDecoyAccess}>
            检查访问
          </Button>
          <Button variant="outline" onClick={onCleanupDecoy}>
            清除假配置
          </Button>
        </div>
      </div>
    </div>
  )
}
