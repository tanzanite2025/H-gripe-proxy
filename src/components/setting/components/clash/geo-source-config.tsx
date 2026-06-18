import { useLockFn } from 'ahooks'
import { Globe } from 'lucide-react'
import type { ChangeEvent, Ref } from 'react'
import { useImperativeHandle, useState } from 'react'

import { BaseDialog, DialogRef } from '@/components/base'
import { TextField } from '@/components/tailwind'
import {
  getRuntimeBaseConfig,
  patchRuntimeBaseConfig,
} from '@/services/core-runtime'
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
      getRuntimeBaseConfig()
        .then((config) => {
          setUrls({
            geoIp: config.geoxUrl.geoIp || '',
            mmdb: config.geoxUrl.mmdb || '',
            asn: config.geoxUrl.asn || '',
            geoSite: config.geoxUrl.geoSite || '',
          })
        })
        .catch(() => {})
    },
    close: () => setOpen(false),
  }))

  const onSave = useLockFn(async () => {
    setSaving(true)
    try {
      await patchRuntimeBaseConfig({
        'geox-url': {
          'geo-ip': urls.geoIp || undefined,
          mmdb: urls.mmdb || undefined,
          asn: urls.asn || undefined,
          'geo-site': urls.geoSite || undefined,
        },
      })
      showNotice.success('GeoData 源配置已保存')
      setOpen(false)
    } catch (error) {
      showNotice.error(error)
    } finally {
      setSaving(false)
    }
  })

  const onClose = () => setOpen(false)
  const updateField =
    (key: keyof GeoXUrlState) => (event: ChangeEvent<HTMLInputElement>) => {
      setUrls((current) => ({ ...current, [key]: event.target.value }))
    }

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
        <div className="mb-2 flex items-center gap-2">
          <Globe className="h-4 w-4 text-cyan-500" />
          <span className="text-sm text-text-secondary">
            留空表示使用默认源，自定义源需要填写完整下载链接。
          </span>
        </div>

        <TextField
          label="GeoIP 源 (dat 格式)"
          placeholder="https://example.com/GeoIP.dat"
          value={urls.geoIp}
          onChange={updateField('geoIp')}
          size="small"
          className="w-full"
        />

        <TextField
          label="MMDB 源 (GeoIP 数据库)"
          placeholder="https://example.com/country.mmdb"
          value={urls.mmdb}
          onChange={updateField('mmdb')}
          size="small"
          className="w-full"
        />

        <TextField
          label="ASN 源"
          placeholder="https://example.com/ASN.mmdb"
          value={urls.asn}
          onChange={updateField('asn')}
          size="small"
          className="w-full"
        />

        <TextField
          label="GeoSite 源 (dat 格式)"
          placeholder="https://example.com/GeoSite.dat"
          value={urls.geoSite}
          onChange={updateField('geoSite')}
          size="small"
          className="w-full"
        />
      </div>
    </BaseDialog>
  )
}
