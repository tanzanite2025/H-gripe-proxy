import { useLockFn } from 'ahooks'
import { Globe } from 'lucide-react'
import type { Ref } from 'react'
import { useImperativeHandle, useState } from 'react'
import { getBaseConfig, patchBaseConfig } from 'tauri-plugin-mihomo-api'

import { BaseDialog, DialogRef } from '@/components/base'
import { TextField } from '@/components/tailwind'
import { showNotice } from '@/services/notice-service'

interface GeoXUrlState {
  geoIp: string
  mmdb: string
  asn: string
  geoSite: string
}

const DEFAULT_URLS: GeoXUrlState = {
  geoIp: '',
  mmdb: '',
  asn: '',
  geoSite: '',
}

export function GeoSourceConfig({ ref }: { ref?: Ref<DialogRef> }) {
  const [open, setOpen] = useState(false)
  const [urls, setUrls] = useState<GeoXUrlState>(DEFAULT_URLS)
  const [saving, setSaving] = useState(false)

  useImperativeHandle(ref, () => ({
    open: () => {
      setOpen(true)
      getBaseConfig()
        .then((cfg) => {
          setUrls({
            geoIp: cfg.geoxUrl.geoIp || '',
            mmdb: cfg.geoxUrl.mmdb || '',
            asn: cfg.geoxUrl.asn || '',
            geoSite: cfg.geoxUrl.geoSite || '',
          })
        })
        .catch(() => {})
    },
    close: () => setOpen(false),
  }))

  const onSave = useLockFn(async () => {
    setSaving(true)
    try {
      await patchBaseConfig({
        'geox-url': {
          'geo-ip': urls.geoIp || undefined,
          mmdb: urls.mmdb || undefined,
          asn: urls.asn || undefined,
          'geo-site': urls.geoSite || undefined,
        },
      })
      showNotice.success('GeoData 源配置已保存')
      setOpen(false)
    } catch (err: any) {
      showNotice.error(err)
    } finally {
      setSaving(false)
    }
  })

  const onClose = () => setOpen(false)

  return (
    <BaseDialog
      title="GeoData 自定义源"
      open={open}
      onClose={onClose}
      onOk={onSave}
      okBtn="保存"
      onCancel={onClose}
      cancelBtn="取消"
      disableOk={saving}
      loading={saving}
    >
      <div className="space-y-4 pt-2">
        <div className="flex items-center gap-2 mb-2">
          <Globe className="h-4 w-4 text-cyan-500" />
          <span className="text-sm text-text-secondary">
            留空使用默认源，自定义源需为完整下载链接
          </span>
        </div>

        <TextField
          label="GeoIP 源 (dat 格式)"
          placeholder="https://example.com/GeoIP.dat"
          value={urls.geoIp}
          onChange={(e: React.ChangeEvent<HTMLInputElement>) => setUrls({ ...urls, geoIp: e.target.value })}
          size="small"
          className="w-full"
        />

        <TextField
          label="MMDB 源 (GeoIP 数据库)"
          placeholder="https://example.com/country.mmdb"
          value={urls.mmdb}
          onChange={(e: React.ChangeEvent<HTMLInputElement>) => setUrls({ ...urls, mmdb: e.target.value })}
          size="small"
          className="w-full"
        />

        <TextField
          label="ASN 源"
          placeholder="https://example.com/ASN.mmdb"
          value={urls.asn}
          onChange={(e: React.ChangeEvent<HTMLInputElement>) => setUrls({ ...urls, asn: e.target.value })}
          size="small"
          className="w-full"
        />

        <TextField
          label="GeoSite 源 (dat 格式)"
          placeholder="https://example.com/GeoSite.dat"
          value={urls.geoSite}
          onChange={(e: React.ChangeEvent<HTMLInputElement>) => setUrls({ ...urls, geoSite: e.target.value })}
          size="small"
          className="w-full"
        />
      </div>
    </BaseDialog>
  )
}
