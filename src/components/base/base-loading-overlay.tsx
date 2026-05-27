import { Loader2 } from 'lucide-react'
import React from 'react'

interface BaseLoadingOverlayProps {
  isLoading: boolean
}

export const BaseLoadingOverlay: React.FC<BaseLoadingOverlayProps> = ({
  isLoading,
}) => {
  if (!isLoading) return null

  return (
    <div className="absolute inset-0 z-[1000] flex items-center justify-center bg-white/70 dark:bg-black/50">
      <Loader2 className="h-10 w-10 animate-spin text-primary dark:text-primary-dark-mode" />
    </div>
  )
}
