import React, { ReactNode } from 'react'

import { BaseErrorBoundary } from './base-error-boundary'

interface Props {
  title?: React.ReactNode // the page title
  header?: React.ReactNode // something behind title
  contentStyle?: React.CSSProperties
  children?: ReactNode
  full?: boolean
}

export const BasePage: React.FC<Props> = (props) => {
  const { title, header, contentStyle, full, children } = props

  return (
    <BaseErrorBoundary>
      <div className="base-page">
        <header
          className="base-page__header"
          data-tauri-drag-region="true"
          style={{ userSelect: 'none' }}
        >
          <h1
            className="uds-title-h1 base-page__title text-lg font-bold"
            data-tauri-drag-region="true"
          >
            {title}
          </h1>

          {header ? <div className="base-page__header-actions">{header}</div> : null}
        </header>

        <div
          className={full ? 'base-container no-padding uds-surface' : 'base-container uds-surface'}
        >
          <section className="base-page__section">
            <div className="base-content base-page__content" style={contentStyle}>
              {children}
            </div>
          </section>
        </div>
      </div>
    </BaseErrorBoundary>
  )
}
