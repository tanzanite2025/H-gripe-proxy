import { motion, AnimatePresence } from 'framer-motion'
import { useState, type ReactNode } from 'react'

export interface TooltipProps {
  content?: string
  title?: string // MUI compatibility
  placement?: 'top' | 'bottom' | 'left' | 'right'
  arrow?: boolean
  children: ReactNode
}

export const Tooltip = ({ content, title, placement = 'top', arrow = false, children }: TooltipProps) => {
  const [isVisible, setIsVisible] = useState(false)
  
  // Support both content and title props for MUI compatibility
  const tooltipText = title || content

  if (!tooltipText) {
    return <>{children}</>
  }

  const placementClasses = {
    top: 'bottom-full left-1/2 -translate-x-1/2 mb-2',
    bottom: 'top-full left-1/2 -translate-x-1/2 mt-2',
    left: 'right-full top-1/2 -translate-y-1/2 mr-2',
    right: 'left-full top-1/2 -translate-y-1/2 ml-2',
  }

  const arrowClasses = {
    top: 'top-full left-1/2 -translate-x-1/2 border-t-gray-900 dark:border-t-gray-700',
    bottom: 'bottom-full left-1/2 -translate-x-1/2 border-b-gray-900 dark:border-b-gray-700',
    left: 'left-full top-1/2 -translate-y-1/2 border-l-gray-900 dark:border-l-gray-700',
    right: 'right-full top-1/2 -translate-y-1/2 border-r-gray-900 dark:border-r-gray-700',
  }

  return (
    <div
      className="relative inline-block"
      onMouseEnter={() => setIsVisible(true)}
      onMouseLeave={() => setIsVisible(false)}
      onFocus={() => setIsVisible(true)}
      onBlur={() => setIsVisible(false)}
    >
      {children}
      <AnimatePresence>
        {isVisible && (
          <motion.div
            initial={{ opacity: 0, scale: 0.95 }}
            animate={{ opacity: 1, scale: 1 }}
            exit={{ opacity: 0, scale: 0.95 }}
            transition={{ duration: 0.15 }}
            className={`absolute z-50 ${placementClasses[placement]}`}
            role="tooltip"
          >
            <div className="rounded-lg bg-gray-900 dark:bg-gray-700 px-3 py-2 text-xs font-black uppercase tracking-wider text-white shadow-lg">
              {tooltipText}
            </div>
            {arrow && (
              <div
                className={`absolute h-0 w-0 border-4 border-transparent ${arrowClasses[placement]}`}
              />
            )}
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  )
}

Tooltip.displayName = 'Tooltip'
