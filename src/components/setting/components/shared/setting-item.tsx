import { ChevronRight } from 'lucide-react'
import React, { ReactNode, useState } from 'react'

import { Box, CircularProgress } from '@/components/tailwind'
import isAsyncFunction from '@/utils/misc/is-async-function'

interface ItemProps {
  label: ReactNode
  extra?: ReactNode
  children?: ReactNode
  secondary?: ReactNode
  onClick?: () => void | Promise<any>
}

export const SettingItem: React.FC<ItemProps> = ({
  label,
  extra,
  children,
  secondary,
  onClick,
}) => {
  const clickable = !!onClick

  const primary = (
    <Box className="uds-settings-item__label-row">
      <Box as="span" className="uds-settings-item__label uds-card-title">
        {label}
      </Box>
      {extra ? <Box className="uds-settings-item__extra">{extra}</Box> : null}
    </Box>
  )
  const secondaryContent = secondary ? (
    <Box className="uds-settings-item__secondary uds-desc">{secondary}</Box>
  ) : null

  const [isLoading, setIsLoading] = useState(false)
  const handleClick = () => {
    if (onClick) {
      if (isAsyncFunction(onClick)) {
        setIsLoading(true)
        onClick()!.finally(() => setIsLoading(false))
      } else {
        onClick()
      }
    }
  }

  return clickable ? (
    <Box as="li" className="uds-settings-item uds-settings-item--clickable p-0">
      <Box
        as="button"
        className="uds-settings-item__button w-full text-left"
        onClick={handleClick}
        disabled={isLoading}
        role="button"
        tabIndex={0}
        onKeyDown={(e: React.KeyboardEvent) => {
          if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault()
            handleClick()
          }
        }}
      >
        <Box className="uds-settings-item__body">
          <Box className="uds-settings-item__main">
            {primary}
            {secondaryContent}
          </Box>
          <Box className="uds-settings-item__action">
            {isLoading ? (
              <CircularProgress color="inherit" size={20} />
            ) : (
              <ChevronRight size={20} />
            )}
          </Box>
        </Box>
      </Box>
    </Box>
  ) : (
    <Box as="li" className="uds-settings-item p-0">
      <Box className="uds-settings-item__body">
        <Box className="uds-settings-item__main">
          {primary}
          {secondaryContent}
        </Box>
        {children ? <Box className="uds-settings-item__control">{children}</Box> : null}
      </Box>
    </Box>
  )
}

export const SettingList: React.FC<{
  title: string
  children: ReactNode
}> = ({ title, children }) => (
  <Box as="ul" className="uds-settings-list p-0">
    <Box
      as="div"
      className="uds-label uds-settings-list__header"
      role="heading"
      aria-level={2}
    >
      {title}
    </Box>

    {children}
  </Box>
)
