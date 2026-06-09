import { useTranslation } from 'react-i18next'

import { BaseDialog } from '@/components/base'
import { EditorViewer } from '@/components/profile/editor-viewer'
import { ProxiesEditorViewer } from '@/components/profile/proxies-editor-viewer'
import { QrViewer } from '@/components/profile/qr-viewer'
import { RulesEditorViewer } from '@/components/profile/rules-editor-viewer'
import { Typography } from '@/components/tailwind/Typography'

import {
  buildProfileQrCodeValue,
  type ProfileItemProps,
} from './shared'
import type { ProfileItemDialogsController } from './use-profile-item-dialogs'

interface ProfileItemDialogsProps {
  itemData: IProfileItem
  name: string
  option?: IProfileOption
  onSave?: ProfileItemProps['onSave']
  onDelete: ProfileItemProps['onDelete']
  dialogs: ProfileItemDialogsController
}

export function ProfileItemDialogs({
  itemData,
  name,
  option,
  onSave,
  onDelete,
  dialogs,
}: ProfileItemDialogsProps) {
  const { t } = useTranslation()

  return (
    <>
      {dialogs.fileOpen && (
        <EditorViewer
          open={true}
          value={dialogs.profileDocument.value}
          language="yaml"
          path={`profile:${itemData.uid}.yaml`}
          loading={dialogs.profileDocument.loading}
          dirty={dialogs.profileDocument.dirty}
          onChange={dialogs.profileDocument.setValue}
          onSave={dialogs.handleSaveProfileDocument}
          onClose={dialogs.closeFile}
        />
      )}

      {dialogs.rulesOpen && (
        <RulesEditorViewer
          groupsUid={option?.groups ?? ''}
          mergeUid={option?.merge ?? ''}
          profileUid={itemData.uid}
          property={option?.rules ?? ''}
          open={true}
          onSave={onSave}
          onClose={dialogs.closeRules}
        />
      )}

      {dialogs.proxiesOpen && (
        <ProxiesEditorViewer
          profileUid={itemData.uid}
          property={option?.proxies ?? ''}
          open={true}
          onSave={onSave}
          onClose={dialogs.closeProxies}
        />
      )}

      {dialogs.scriptOpen && (
        <EditorViewer
          open={true}
          value={dialogs.scriptDocument.value}
          language="javascript"
          path={`script:${option?.script ?? ''}.js`}
          loading={dialogs.scriptDocument.loading}
          dirty={dialogs.scriptDocument.dirty}
          onChange={dialogs.scriptDocument.setValue}
          onSave={dialogs.handleSaveScriptDocument}
          onClose={dialogs.closeScript}
        />
      )}

      <BaseDialog
        title={t('profiles.modals.confirmDelete.title')}
        open={dialogs.confirmOpen}
        okBtn={t('shared.actions.confirm')}
        cancelBtn={t('shared.actions.cancel')}
        panelStyle={{ width: 'min(420px, calc(100vw - 56px))' }}
        contentClassName="select-text"
        onCancel={dialogs.closeConfirm}
        onClose={dialogs.closeConfirm}
        onOk={() => {
          void onDelete()
          dialogs.closeConfirm()
        }}
      >
        <Typography className="break-words">
          {t('profiles.modals.confirmDelete.message')}
        </Typography>
      </BaseDialog>

      {dialogs.qrOpen && itemData.url && (
        <QrViewer
          open={true}
          value={buildProfileQrCodeValue(itemData.url, name)}
          subject={name}
          fileName={name}
          onClose={dialogs.closeQr}
        />
      )}
    </>
  )
}
