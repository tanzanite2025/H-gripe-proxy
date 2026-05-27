import React, { createContext, useContext } from 'react'

interface TabsContextValue {
  value: string | number
  onChange: (event: React.SyntheticEvent, newValue: string | number) => void
}

const TabsContext = createContext<TabsContextValue | undefined>(undefined)

export interface TabsProps {
  children?: React.ReactNode
  value: string | number
  onChange: (event: React.SyntheticEvent, newValue: string | number) => void
  className?: string
  variant?: 'standard' | 'scrollable'
  scrollButtons?: 'auto' | 'desktop' | 'on' | 'off'
  textColor?: 'primary' | 'secondary' | 'inherit'
  indicatorColor?: 'primary' | 'secondary'
}

export const Tabs = React.forwardRef<HTMLDivElement, TabsProps>(
  ({ children, value, onChange, className = '', variant = 'standard', textColor, indicatorColor, ...props }, ref) => {
    return (
      <TabsContext.Provider value={{ value, onChange }}>
        <div
          ref={ref}
          className={`flex ${variant === 'scrollable' ? 'overflow-x-auto' : ''} ${className}`}
          role="tablist"
          {...props}
        >
          {children}
        </div>
      </TabsContext.Provider>
    )
  }
)

Tabs.displayName = 'Tabs'

export interface TabProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  label?: string
  value?: string | number
  disabled?: boolean
  className?: string
}

export const Tab = React.forwardRef<HTMLButtonElement, TabProps>(
  ({ label, value: tabValue, disabled, className = '', ...props }, ref) => {
    const context = useContext(TabsContext)

    if (!context) {
      throw new Error('Tab must be used within Tabs')
    }

    const { value: selectedValue, onChange } = context
    const isSelected = tabValue !== undefined ? selectedValue === tabValue : false

    return (
      <button
        ref={ref}
        className={`px-4 py-2 font-medium transition-colors border-b-2 ${
          isSelected
            ? 'border-blue-500 text-blue-500'
            : 'border-transparent text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-100'
        } ${disabled ? 'opacity-50 cursor-not-allowed' : ''} ${className}`}
        role="tab"
        aria-selected={isSelected}
        disabled={disabled}
        onClick={(e) => !disabled && tabValue !== undefined && onChange(e, tabValue)}
        {...props}
      >
        {label}
      </button>
    )
  }
)

Tab.displayName = 'Tab'
