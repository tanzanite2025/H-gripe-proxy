import React from 'react'
import { motion, AnimatePresence } from 'framer-motion'

export interface ZoomProps {
  children: React.ReactElement
  in: boolean
  unmountOnExit?: boolean
}

export const Zoom: React.FC<ZoomProps> = ({ children, in: inProp, unmountOnExit = false }) => {
  if (!unmountOnExit && !inProp) {
    return <div style={{ opacity: 0, transform: 'scale(0)' }}>{children}</div>
  }

  return (
    <AnimatePresence>
      {inProp && (
        <motion.div
          initial={{ opacity: 0, scale: 0 }}
          animate={{ opacity: 1, scale: 1 }}
          exit={{ opacity: 0, scale: 0 }}
          transition={{ duration: 0.2 }}
        >
          {children}
        </motion.div>
      )}
    </AnimatePresence>
  )
}
