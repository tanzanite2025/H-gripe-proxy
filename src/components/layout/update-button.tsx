import { useRef } from 'react'

import { DialogRef } from '@/components/base'
import { Button } from '@/components/tailwind'
import { useUpdate } from '@/hooks/system'

import { UpdateViewer } from '../setting/components/misc/update-config'

interface Props {
  className?: string
}

export const UpdateButton = (props: Props) => {
  const { className } = props
  const viewerRef = useRef<DialogRef>(null)

  const { updateInfo } = useUpdate()

  if (!updateInfo?.available) return null

  return (
    <>
      <UpdateViewer ref={viewerRef} />

      <Button
        variant="danger"
        size="small"
        className={className}
        onClick={() => viewerRef.current?.open()}
      >
        New
      </Button>
    </>
  )
}
