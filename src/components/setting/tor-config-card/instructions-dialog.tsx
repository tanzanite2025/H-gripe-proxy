import { Button } from '@/components/tailwind/Button'
import {
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
} from '@/components/tailwind/Dialog'

import { TOR_USAGE_INSTRUCTIONS } from './constants'

interface InstructionsDialogProps {
  open: boolean
  onClose: () => void
}

export function InstructionsDialog({
  open,
  onClose,
}: InstructionsDialogProps) {
  return (
    <Dialog open={open} onClose={onClose} maxWidth="sm" fullWidth>
      <DialogTitle>{TOR_USAGE_INSTRUCTIONS.title}</DialogTitle>
      <DialogContent>
        <div className="space-y-4">
          <div>
            <div className="mb-2 text-sm font-medium text-primary">
              配置步骤
            </div>
            <ol className="space-y-2 pl-5 text-sm text-text-primary">
              {TOR_USAGE_INSTRUCTIONS.steps.map((step) => (
                <li key={step} className="list-decimal">
                  {step}
                </li>
              ))}
            </ol>
          </div>

          <div>
            <div className="mb-2 text-sm font-medium text-warning">
              注意事项
            </div>
            <ul className="space-y-2 pl-5 text-sm text-text-primary">
              {TOR_USAGE_INSTRUCTIONS.notes.map((note) => (
                <li key={note} className="list-disc">
                  {note}
                </li>
              ))}
            </ul>
          </div>
        </div>
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose}>关闭</Button>
      </DialogActions>
    </Dialog>
  )
}
