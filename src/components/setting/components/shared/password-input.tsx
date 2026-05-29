import { useState, type ChangeEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { Eye, EyeOff } from 'lucide-react'
import { Button } from '@/components/tailwind/Button'
import { Dialog } from '@/components/tailwind/Dialog'
import { TextField } from '@/components/tailwind/TextField'
import { IconButton } from '@/components/tailwind/IconButton'

interface Props {
  onConfirm: (passwd: string) => Promise<void>
}

export const PasswordInput = (props: Props) => {
  const { onConfirm } = props

  const { t } = useTranslation()
  const [passwd, setPasswd] = useState('')
  const [showPassword, setShowPassword] = useState(false)

  const handleTogglePasswordVisibility = () => {
    setShowPassword((prev) => !prev)
  }

  return (
    <Dialog
      open={true}
      onClose={() => {}}
      title={t('settings.modals.password.prompts.enterRoot')}
      maxWidth="sm"
      showCloseButton={false}
      actions={
        <Button
          onClick={async () => await onConfirm(passwd)}
          variant="primary"
        >
          {t('shared.actions.confirm')}
        </Button>
      }
    >
      <div className="relative mt-1">
        <TextField
          autoFocus
          label={t('shared.labels.password')}
          type={showPassword ? 'text' : 'password'}
          value={passwd}
          onKeyDown={(e) => e.key === 'Enter' && onConfirm(passwd)}
          onChange={(e: ChangeEvent<HTMLInputElement>) => setPasswd(e.target.value)}
        />
        <div className="absolute right-3 top-9">
          <IconButton
            size="small"
            onClick={handleTogglePasswordVisibility}
            aria-label={showPassword ? 'Hide password' : 'Show password'}
            type="button"
          >
            {showPassword ? (
              <EyeOff className="h-4 w-4 text-gray-500 dark:text-gray-400" />
            ) : (
              <Eye className="h-4 w-4 text-gray-500 dark:text-gray-400" />
            )}
          </IconButton>
        </div>
      </div>
    </Dialog>
  )
}
