import { Copy } from 'lucide-react'

import { Button, TextField } from '@/components/tailwind'

interface SecureStoragePanelProps {
  encryptionKey: string
  hasEncryptionKey: boolean
  onGenerateKey: () => void
  onCopyKey: () => void
}

export default function SecureStoragePanel({
  encryptionKey,
  hasEncryptionKey,
  onGenerateKey,
  onCopyKey,
}: SecureStoragePanelProps) {
  return (
    <div className="p-4 bg-card border border-border rounded-lg">
      <h3 className="text-sm font-semibold mb-4">加密密钥管理</h3>
      <div className="space-y-4">
        <div className="flex items-center gap-2">
          <div
            className={`inline-flex items-center gap-1 px-3 py-1 rounded-full text-sm ${
              hasEncryptionKey ? 'bg-green-500 text-white' : 'bg-yellow-500 text-white'
            }`}
          >
            <span>{hasEncryptionKey ? '密钥已设置' : '密钥未设置'}</span>
          </div>
          <p className="text-xs text-muted-foreground">
            真实配置只在内存中加密存储
          </p>
        </div>

        <Button variant="default" onClick={onGenerateKey} className="w-full">
          生成新密钥
        </Button>

        {encryptionKey && (
          <div className="space-y-2">
            <div className="relative">
              <TextField
                label="加密密钥（请保存到环境变量）"
                value={encryptionKey}
                fullWidth
                readOnly
                className="font-mono text-xs"
              />
              <Button
                size="sm"
                variant="ghost"
                onClick={onCopyKey}
                className="absolute right-2 top-8"
              >
                <Copy className="w-4 h-4 mr-1" />
                复制
              </Button>
            </div>
            <p className="text-xs text-yellow-600 dark:text-yellow-400">
              请将此密钥设置为环境变量 CLASH_VERGE_SECURE_KEY
            </p>
          </div>
        )}
      </div>
    </div>
  )
}
