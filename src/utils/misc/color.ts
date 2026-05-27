/**
 * Color utility functions
 */

/**
 * Convert hex color to rgba with alpha
 */
export const alpha = (color: string, opacity: number): string => {
  // Remove # if present
  const hex = color.replace('#', '')
  
  // Parse hex to RGB
  let r: number, g: number, b: number
  
  if (hex.length === 3) {
    // Short hex format (#RGB)
    r = parseInt(hex[0] + hex[0], 16)
    g = parseInt(hex[1] + hex[1], 16)
    b = parseInt(hex[2] + hex[2], 16)
  } else if (hex.length === 6) {
    // Full hex format (#RRGGBB)
    r = parseInt(hex.substring(0, 2), 16)
    g = parseInt(hex.substring(2, 4), 16)
    b = parseInt(hex.substring(4, 6), 16)
  } else {
    // Invalid format, return original
    return color
  }
  
  // Return rgba string
  return `rgba(${r}, ${g}, ${b}, ${opacity})`
}

/**
 * Darken a color by a percentage
 */
export const darken = (color: string, amount: number): string => {
  const hex = color.replace('#', '')
  
  let r: number, g: number, b: number
  
  if (hex.length === 3) {
    r = parseInt(hex[0] + hex[0], 16)
    g = parseInt(hex[1] + hex[1], 16)
    b = parseInt(hex[2] + hex[2], 16)
  } else if (hex.length === 6) {
    r = parseInt(hex.substring(0, 2), 16)
    g = parseInt(hex.substring(2, 4), 16)
    b = parseInt(hex.substring(4, 6), 16)
  } else {
    return color
  }
  
  // Darken by reducing RGB values
  r = Math.max(0, Math.floor(r * (1 - amount)))
  g = Math.max(0, Math.floor(g * (1 - amount)))
  b = Math.max(0, Math.floor(b * (1 - amount)))
  
  // Convert back to hex
  const toHex = (n: number) => n.toString(16).padStart(2, '0')
  return `#${toHex(r)}${toHex(g)}${toHex(b)}`
}

/**
 * Lighten a color by a percentage
 */
export const lighten = (color: string, amount: number): string => {
  const hex = color.replace('#', '')
  
  let r: number, g: number, b: number
  
  if (hex.length === 3) {
    r = parseInt(hex[0] + hex[0], 16)
    g = parseInt(hex[1] + hex[1], 16)
    b = parseInt(hex[2] + hex[2], 16)
  } else if (hex.length === 6) {
    r = parseInt(hex.substring(0, 2), 16)
    g = parseInt(hex.substring(2, 4), 16)
    b = parseInt(hex.substring(4, 6), 16)
  } else {
    return color
  }
  
  // Lighten by increasing RGB values
  r = Math.min(255, Math.floor(r + (255 - r) * amount))
  g = Math.min(255, Math.floor(g + (255 - g) * amount))
  b = Math.min(255, Math.floor(b + (255 - b) * amount))
  
  // Convert back to hex
  const toHex = (n: number) => n.toString(16).padStart(2, '0')
  return `#${toHex(r)}${toHex(g)}${toHex(b)}`
}
