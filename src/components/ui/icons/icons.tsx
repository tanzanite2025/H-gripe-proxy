import type { ReactNode, SVGProps } from 'react'
import { forwardRef } from 'react'

export interface UdsIconProps extends SVGProps<SVGSVGElement> {
  size?: number | string
}

interface UdsIconBaseProps extends UdsIconProps {
  children: ReactNode
  viewBox?: string
}

const UdsIconBase = forwardRef<SVGSVGElement, UdsIconBaseProps>(function UdsIconBase(
  { size = 18, viewBox = '0 0 24 24', children, style, ...props },
  ref,
) {
  return (
    <svg
      ref={ref}
      width={size}
      height={size}
      viewBox={viewBox}
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      aria-hidden="true"
      focusable="false"
      style={{ display: 'block', flex: '0 0 auto', ...style }}
      {...props}
    >
      {children}
    </svg>
  )
})

export const UdsQuestionIcon = forwardRef<SVGSVGElement, UdsIconProps>(
  function UdsQuestionIcon(props, ref) {
    return (
      <UdsIconBase ref={ref} {...props}>
        <circle cx="12" cy="12" r="9" stroke="currentColor" strokeWidth="1.8" />
        <path d="M9.8 9.4a2.5 2.5 0 0 1 4.9.8c0 1.9-2.3 2.2-2.3 4" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" />
        <circle cx="12" cy="17.4" r="1" fill="currentColor" />
      </UdsIconBase>
    )
  },
)

export const UdsSparkIcon = forwardRef<SVGSVGElement, UdsIconProps>(
  function UdsSparkIcon(props, ref) {
    return (
      <UdsIconBase ref={ref} {...props}>
        <path d="M12 3.5L13.8 9l5.7 1.8-5.7 1.8L12 18l-1.8-5.4L4.5 10.8 10.2 9 12 3.5Z" fill="currentColor" />
      </UdsIconBase>
    )
  },
)

export const UdsSettingsIcon = forwardRef<SVGSVGElement, UdsIconProps>(
  function UdsSettingsIcon(props, ref) {
    return (
      <UdsIconBase ref={ref} {...props}>
        <path d="M12 4.8 13.5 3l2 1.2-.3 2.3a6.7 6.7 0 0 1 1.5 1.5l2.3-.3L20.2 10 18.4 11.5a6.9 6.9 0 0 1 0 2.1l1.8 1.5-1.2 2.3-2.3-.3a6.7 6.7 0 0 1-1.5 1.5l.3 2.3-2 1.2L12 19.2a6.9 6.9 0 0 1-2.1 0L8.4 21l-2-1.2.3-2.3a6.7 6.7 0 0 1-1.5-1.5l-2.3.3L1.8 14l1.8-1.5a6.9 6.9 0 0 1 0-2.1L1.8 9l1.1-2.3 2.3.3a6.7 6.7 0 0 1 1.5-1.5L6.4 3.2l2-1.2 1.5 1.8a6.9 6.9 0 0 1 2.1 0Z" stroke="currentColor" strokeWidth="1.6" strokeLinejoin="round" />
        <circle cx="12" cy="12" r="2.6" stroke="currentColor" strokeWidth="1.6" />
      </UdsIconBase>
    )
  },
)

export const UdsCloudUploadIcon = forwardRef<SVGSVGElement, UdsIconProps>(
  function UdsCloudUploadIcon(props, ref) {
    return (
      <UdsIconBase ref={ref} {...props}>
        <path d="M7.5 18.2h8.8a3.7 3.7 0 0 0 .6-7.3 5.5 5.5 0 0 0-10.3-1.6A3.8 3.8 0 0 0 7.5 18.2Z" stroke="currentColor" strokeWidth="1.8" strokeLinejoin="round" />
        <path d="M12 15.6V9.8" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
        <path d="m9.5 12.2 2.5-2.6 2.5 2.6" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" />
      </UdsIconBase>
    )
  },
)

export const UdsDnsIcon = forwardRef<SVGSVGElement, UdsIconProps>(function UdsDnsIcon(
  props,
  ref,
) {
  return (
    <UdsIconBase ref={ref} {...props}>
      <rect x="5" y="5.5" width="14" height="5" rx="1.5" stroke="currentColor" strokeWidth="1.7" />
      <rect x="5" y="13.5" width="14" height="5" rx="1.5" stroke="currentColor" strokeWidth="1.7" />
      <circle cx="8.2" cy="8" r="0.9" fill="currentColor" />
      <circle cx="8.2" cy="16" r="0.9" fill="currentColor" />
    </UdsIconBase>
  )
})

export const UdsRouteIcon = forwardRef<SVGSVGElement, UdsIconProps>(
  function UdsRouteIcon(props, ref) {
    return (
      <UdsIconBase ref={ref} {...props}>
        <circle cx="6.5" cy="7" r="2" fill="currentColor" />
        <circle cx="17.5" cy="12" r="2" fill="currentColor" />
        <circle cx="8.5" cy="17.5" r="2" fill="currentColor" />
        <path d="M8 7.8c2 0 3.8 1 4.9 2.7m-3.2 5.5c1.2-1.1 2-2.3 2.6-3.7" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
      </UdsIconBase>
    )
  },
)

export const UdsSpeedIcon = forwardRef<SVGSVGElement, UdsIconProps>(
  function UdsSpeedIcon(props, ref) {
    return (
      <UdsIconBase ref={ref} {...props}>
        <path d="M5.4 16a6.7 6.7 0 1 1 13.2 0" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
        <path d="m12 12 4.2-2.2" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
        <circle cx="12" cy="12" r="1.4" fill="currentColor" />
      </UdsIconBase>
    )
  },
)

export const UdsArrowUpIcon = forwardRef<SVGSVGElement, UdsIconProps>(
  function UdsArrowUpIcon(props, ref) {
    return (
      <UdsIconBase ref={ref} {...props}>
        <path d="M12 19V6" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" />
        <path d="m7 11 5-5 5 5" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round" />
      </UdsIconBase>
    )
  },
)

export const UdsArrowDownIcon = forwardRef<SVGSVGElement, UdsIconProps>(
  function UdsArrowDownIcon(props, ref) {
    return (
      <UdsIconBase ref={ref} {...props}>
        <path d="M12 5v13" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" />
        <path d="m7 13 5 5 5-5" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round" />
      </UdsIconBase>
    )
  },
)

export const UdsMemoryIcon = forwardRef<SVGSVGElement, UdsIconProps>(
  function UdsMemoryIcon(props, ref) {
    return (
      <UdsIconBase ref={ref} {...props}>
        <rect x="7" y="7" width="10" height="10" rx="1.8" stroke="currentColor" strokeWidth="1.7" />
        <rect x="10" y="10" width="4" height="4" rx="0.8" fill="currentColor" />
        <path d="M4.5 9h2M4.5 15h2M17.5 9h2M17.5 15h2M9 4.5v2M15 4.5v2M9 17.5v2M15 17.5v2" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" />
      </UdsIconBase>
    )
  },
)

export const UdsCalendarIcon = forwardRef<SVGSVGElement, UdsIconProps>(
  function UdsCalendarIcon(props, ref) {
    return (
      <UdsIconBase ref={ref} {...props}>
        <rect x="4.5" y="6.5" width="15" height="13" rx="2" stroke="currentColor" strokeWidth="1.7" />
        <path d="M8 4.8v3.4M16 4.8v3.4M4.8 10.2h14.4" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" />
      </UdsIconBase>
    )
  },
)

export const UdsLaunchIcon = forwardRef<SVGSVGElement, UdsIconProps>(
  function UdsLaunchIcon(props, ref) {
    return (
      <UdsIconBase ref={ref} {...props}>
        <path d="M13 5h6v6" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" />
        <path d="M10 14 19 5" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
        <path d="M19 13v4a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V7a2 2 0 0 1 2-2h4" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" />
      </UdsIconBase>
    )
  },
)

export const UdsStorageIcon = forwardRef<SVGSVGElement, UdsIconProps>(
  function UdsStorageIcon(props, ref) {
    return (
      <UdsIconBase ref={ref} {...props}>
        <ellipse cx="12" cy="6.7" rx="5.8" ry="2.7" stroke="currentColor" strokeWidth="1.7" />
        <path d="M6.2 6.8v5.7c0 1.5 2.6 2.7 5.8 2.7s5.8-1.2 5.8-2.7V6.8" stroke="currentColor" strokeWidth="1.7" />
        <path d="M6.2 12.5v4c0 1.5 2.6 2.7 5.8 2.7s5.8-1.2 5.8-2.7v-4" stroke="currentColor" strokeWidth="1.7" />
      </UdsIconBase>
    )
  },
)

export const UdsUpdateIcon = forwardRef<SVGSVGElement, UdsIconProps>(
  function UdsUpdateIcon(props, ref) {
    return (
      <UdsIconBase ref={ref} {...props}>
        <path d="M18 8.2V4.8h-3.4" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" />
        <path d="M18 4.8a7 7 0 1 0 1.2 7.2" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
      </UdsIconBase>
    )
  },
)

export const UdsWindowCloseIcon = forwardRef<SVGSVGElement, UdsIconProps>(
  function UdsWindowCloseIcon(props, ref) {
    return (
      <UdsIconBase ref={ref} {...props}>
        <path d="m7 7 10 10M17 7 7 17" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" />
      </UdsIconBase>
    )
  },
)

export const UdsWindowMinimizeIcon = forwardRef<SVGSVGElement, UdsIconProps>(
  function UdsWindowMinimizeIcon(props, ref) {
    return (
      <UdsIconBase ref={ref} {...props}>
        <path d="M6.5 12h11" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" />
      </UdsIconBase>
    )
  },
)

export const UdsWindowMaximizeIcon = forwardRef<SVGSVGElement, UdsIconProps>(
  function UdsWindowMaximizeIcon(props, ref) {
    return (
      <UdsIconBase ref={ref} {...props}>
        <rect x="6.5" y="6.5" width="11" height="11" rx="1.4" stroke="currentColor" strokeWidth="1.8" />
      </UdsIconBase>
    )
  },
)

export const UdsWindowRestoreIcon = forwardRef<SVGSVGElement, UdsIconProps>(
  function UdsWindowRestoreIcon(props, ref) {
    return (
      <UdsIconBase ref={ref} {...props}>
        <path d="M9 9h8v8H9z" stroke="currentColor" strokeWidth="1.7" />
        <path d="M7 15H6.5A1.5 1.5 0 0 1 5 13.5V6.5A1.5 1.5 0 0 1 6.5 5h7A1.5 1.5 0 0 1 15 6.5V7" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" />
      </UdsIconBase>
    )
  },
)
