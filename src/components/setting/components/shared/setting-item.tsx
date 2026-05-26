import { ChevronRightRounded } from '@mui/icons-material'
import {
  Box,
  List,
  ListItem,
  ListItemButton,
  ListSubheader,
} from '@mui/material'
import CircularProgress from '@mui/material/CircularProgress'
import React, { ReactNode, useState } from 'react'

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
      <Box component="span" className="uds-settings-item__label uds-card-title">
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
    <ListItem disablePadding className="uds-settings-item uds-settings-item--clickable">
      <ListItemButton
        className="uds-settings-item__button"
        onClick={handleClick}
        disabled={isLoading}
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
              <ChevronRightRounded />
            )}
          </Box>
        </Box>
      </ListItemButton>
    </ListItem>
  ) : (
    <ListItem className="uds-settings-item" sx={{ p: 0 }}>
      <Box className="uds-settings-item__body">
        <Box className="uds-settings-item__main">
          {primary}
          {secondaryContent}
        </Box>
        {children ? <Box className="uds-settings-item__control">{children}</Box> : null}
      </Box>
    </ListItem>
  )
}

export const SettingList: React.FC<{
  title: string
  children: ReactNode
}> = ({ title, children }) => (
  <List disablePadding className="uds-settings-list">
    <ListSubheader
      className="uds-label uds-settings-list__header"
      sx={{ background: 'transparent' }}
      disableSticky
    >
      {title}
    </ListSubheader>

    {children}
  </List>
)
