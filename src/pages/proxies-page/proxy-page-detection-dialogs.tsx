import { Suspense, lazy } from 'react'

import { Dialog, DialogContent } from '@/components/tailwind/Dialog'
import { Skeleton } from '@/components/tailwind/Skeleton'

const LazyProxyDetectionCard = lazy(() =>
  import('@/components/home/proxy-detection-card').then((module) => ({
    default: module.ProxyDetectionCard,
  })),
)

const LazyDNSLeakCard = lazy(() =>
  import('@/components/home/dns-leak-card').then((module) => ({
    default: module.DNSLeakCard,
  })),
)

interface DetectionDialogProps {
  open: boolean
  onClose: () => void
}

const DetectionDialog = ({
  open,
  onClose,
  children,
}: React.PropsWithChildren<DetectionDialogProps>) => {
  return (
    <Dialog
      open={open}
      onClose={onClose}
      maxWidth="sm"
      fullWidth
      className="p-3"
    >
      <DialogContent className="py-0">
        <Suspense fallback={<Skeleton variant="rectangular" height={280} />}>
          {children}
        </Suspense>
      </DialogContent>
    </Dialog>
  )
}

export const ProxyDetectionDialog = ({
  open,
  onClose,
}: DetectionDialogProps) => (
  <DetectionDialog open={open} onClose={onClose}>
    <LazyProxyDetectionCard />
  </DetectionDialog>
)

export const DnsLeakDialog = ({ open, onClose }: DetectionDialogProps) => (
  <DetectionDialog open={open} onClose={onClose}>
    <LazyDNSLeakCard />
  </DetectionDialog>
)
