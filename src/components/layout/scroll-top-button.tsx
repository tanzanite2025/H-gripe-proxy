import { motion, AnimatePresence } from 'framer-motion'
import { ChevronUp } from 'lucide-react'

import { IconButton } from '@/components/tailwind'

interface Props {
  onClick: () => void
  show: boolean
  className?: string
}

export const ScrollTopButton = ({ onClick, show, className = '' }: Props) => {
  return (
    <AnimatePresence>
      {show && (
        <motion.div
          initial={{ opacity: 0, scale: 0.8 }}
          animate={{ opacity: 1, scale: 1 }}
          exit={{ opacity: 0, scale: 0.8 }}
          transition={{ duration: 0.2 }}
          className={className}
        >
          <IconButton
            onClick={onClick}
            className="bg-black/10 dark:bg-white/10 hover:bg-black/20 dark:hover:bg-white/20"
          >
            <ChevronUp className="h-6 w-6" />
          </IconButton>
        </motion.div>
      )}
    </AnimatePresence>
  )
}
