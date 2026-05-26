import { alpha, Box, styled } from '@mui/material'

export const TestBox = styled(Box)(({ theme, 'aria-selected': selected }) => {
  const { mode, primary, text } = theme.palette
  const key = `${mode}-${!!selected}`

  const backgroundColor =
    mode === 'light' 
      ? alpha(primary.main, 0.05) 
      : 'rgba(255, 255, 255, 0.02)'

  const hoverBg =
    mode === 'light'
      ? alpha(primary.main, 0.1)
      : 'rgba(255, 255, 255, 0.06)'

  const color = {
    'light-true': text.secondary,
    'light-false': text.secondary,
    'dark-true': alpha(text.secondary, 0.65),
    'dark-false': alpha(text.secondary, 0.65),
  }[key]!

  const h2color = {
    'light-true': primary.main,
    'light-false': text.primary,
    'dark-true': primary.main,
    'dark-false': text.primary,
  }[key]!

  return {
    position: 'relative',
    width: '100%',
    display: 'block',
    cursor: 'pointer',
    textAlign: 'left',
    borderRadius: 12,
    border: `1px dashed ${mode === 'light' ? 'rgba(0, 0, 0, 0.06)' : 'rgba(255, 255, 255, 0.04)'}`,
    boxShadow: 'none',
    padding: '8px 16px',
    boxSizing: 'border-box',
    backgroundColor,
    color,
    '& h2': { color: h2color },
    transition: 'all 0.25s cubic-bezier(0.16, 1, 0.3, 1)',
    '&:hover': {
      backgroundColor: hoverBg,
      borderColor: primary.main,
      transform: 'translateY(-2px)',
      boxShadow: mode === 'light'
        ? '0 6px 16px -4px rgba(0, 0, 0, 0.05)'
        : '0 6px 20px -4px rgba(0, 0, 0, 0.3)',
    },
  }
})
