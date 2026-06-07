import { ResidentialPoolPanel } from '@/components/advanced/residential-pool-panel'
import { Button } from '@/components/tailwind/Button'
import { Dialog, DialogActions, DialogContent, DialogTitle } from '@/components/tailwind/Dialog'
import type { ResidentialProxyPool } from '@/services/coordinator'

interface ResidentialPoolDialogProps {
  open: boolean
  config: ResidentialProxyPool
  onChange: (config: ResidentialProxyPool) => void
  onClose: () => void
  onSave: () => Promise<void>
}

export const ResidentialPoolDialog = ({
  open,
  config,
  onChange,
  onClose,
  onSave,
}: ResidentialPoolDialogProps) => {
  return (
    <Dialog open={open} onClose={onClose}>
      <DialogTitle>住宅代理池配置</DialogTitle>
      <DialogContent>
        <ResidentialPoolPanel config={config} onChange={onChange} />
      </DialogContent>
      <DialogActions>
        <Button variant="outlined" onClick={onClose}>
          取消
        </Button>
        <Button onClick={() => void onSave()}>保存</Button>
      </DialogActions>
    </Dialog>
  )
}
