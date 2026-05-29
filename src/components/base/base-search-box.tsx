import { X as ClearRounded } from 'lucide-react'
import {
  ChangeEvent,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react'
import { useTranslation } from 'react-i18next'

import MatchCaseIcon from '@/assets/image/component/match_case.svg?react'
import MatchWholeWordIcon from '@/assets/image/component/match_whole_word.svg?react'
import UseRegularExpressionIcon from '@/assets/image/component/use_regular_expression.svg?react'
import { IconButton } from '@/components/tailwind/IconButton'
import { TextField } from '@/components/tailwind/TextField'
import { Tooltip } from '@/components/tailwind/Tooltip'
import { cn } from '@/utils/cn'
import { buildRegex, compileStringMatcher } from '@/utils/validation/search-matcher'

export type SearchState = {
  text: string
  matchCase: boolean
  matchWholeWord: boolean
  useRegularExpression: boolean
}

type SearchOptionState = Omit<SearchState, 'text'>

type SearchProps = {
  value?: string
  defaultValue?: string
  autoFocus?: boolean
  placeholder?: string
  matchCase?: boolean
  matchWholeWord?: boolean
  useRegularExpression?: boolean
  searchState?: Partial<SearchOptionState>
  onSearch: (match: (content: string) => boolean, state: SearchState) => void
}

const useControllableState = <T,>(options: {
  controlled: T | undefined
  defaultValue: T
}) => {
  const { controlled, defaultValue } = options
  const [uncontrolled, setUncontrolled] = useState(defaultValue)
  const isControlled = controlled !== undefined

  const value = isControlled ? controlled : uncontrolled

  const setValue = useCallback(
    (next: T) => {
      if (!isControlled) setUncontrolled(next)
    },
    [isControlled],
  )

  return [value, setValue] as const
}

export const BaseSearchBox = ({
  value,
  defaultValue,
  autoFocus,
  placeholder,
  searchState,
  matchCase: defaultMatchCase = false,
  matchWholeWord: defaultMatchWholeWord = false,
  useRegularExpression: defaultUseRegularExpression = false,
  onSearch,
}: SearchProps) => {
  const { t } = useTranslation()
  const onSearchRef = useRef(onSearch)
  const lastSearchStateRef = useRef<SearchState | null>(null)

  const [text, setText] = useControllableState<string>({
    controlled: value,
    defaultValue: defaultValue ?? '',
  })

  const [matchCase, setMatchCase] = useControllableState<boolean>({
    controlled: searchState?.matchCase,
    defaultValue: defaultMatchCase,
  })

  const [matchWholeWord, setMatchWholeWord] = useControllableState<boolean>({
    controlled: searchState?.matchWholeWord,
    defaultValue: defaultMatchWholeWord,
  })

  const [useRegularExpression, setUseRegularExpression] =
    useControllableState<boolean>({
      controlled: searchState?.useRegularExpression,
      defaultValue: defaultUseRegularExpression,
    })

  const iconStyle = {
    className: 'h-6 w-6 cursor-pointer',
  }

  useEffect(() => {
    onSearchRef.current = onSearch
  }, [onSearch])

  const emitSearch = useCallback((nextState: SearchState) => {
    const prevState = lastSearchStateRef.current
    const isSameState =
      !!prevState &&
      prevState.text === nextState.text &&
      prevState.matchCase === nextState.matchCase &&
      prevState.matchWholeWord === nextState.matchWholeWord &&
      prevState.useRegularExpression === nextState.useRegularExpression
    if (isSameState) return

    const compiled = compileStringMatcher(nextState.text, nextState)
    onSearchRef.current(compiled.matcher, nextState)

    lastSearchStateRef.current = nextState
  }, [])

  useEffect(() => {
    emitSearch({ text, matchCase, matchWholeWord, useRegularExpression })
  }, [emitSearch, matchCase, matchWholeWord, text, useRegularExpression])

  const effectiveErrorMessage = useMemo(() => {
    if (!useRegularExpression || !text) return ''
    const flags = matchCase ? '' : 'i'
    return buildRegex(text, flags) ? '' : t('shared.validation.invalidRegex')
  }, [matchCase, t, text, useRegularExpression])

  const handleChangeText = (
    e: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
  ) => {
    const nextText = e.target?.value ?? ''
    setText(nextText)
    emitSearch({
      text: nextText,
      matchCase,
      matchWholeWord,
      useRegularExpression,
    })
  }

  const handleToggleUseRegularExpression = () => {
    const next = !useRegularExpression
    setUseRegularExpression(next)
    emitSearch({
      text,
      matchCase,
      matchWholeWord,
      useRegularExpression: next,
    })
  }

  const handleClearInput = () => {
    setText('')
    emitSearch({ text: '', matchCase, matchWholeWord, useRegularExpression })
  }

  const handleToggleMatchCase = () => {
    const next = !matchCase
    setMatchCase(next)
    emitSearch({ text, matchCase: next, matchWholeWord, useRegularExpression })
  }

  const handleToggleMatchWholeWord = () => {
    const next = !matchWholeWord
    setMatchWholeWord(next)
    emitSearch({ text, matchCase, matchWholeWord: next, useRegularExpression })
  }

  return (
    <Tooltip title={effectiveErrorMessage || ''} placement="bottom">
      <TextField
        autoComplete="new-password"
        fullWidth
        size="small"
        variant="outlined"
        autoFocus={autoFocus}
        spellCheck="false"
        placeholder={placeholder ?? t('shared.placeholders.filter')}
        className={cn(
          '[&_input]:px-5 [&_input]:py-[0.65rem]',
          '[&_.MuiInputBase-root]:bg-white [&_.MuiInputBase-root]:pr-1 dark:[&_.MuiInputBase-root]:bg-transparent',
          "[&_svg[aria-label='active']_path]:fill-primary-400",
          "[&_svg[aria-label='inactive']_path]:fill-gray-400",
        )}
        value={text}
        onChange={handleChangeText}
        error={!!effectiveErrorMessage}
        slotProps={{
          input: {
            endAdornment: (
              <div className="flex">
                {!!text && (
                  <Tooltip title={t('shared.placeholders.resetInput')}>
                    <IconButton
                      size="small"
                      {...iconStyle}
                      onClick={handleClearInput}
                    >
                      <ClearRounded className="h-4 w-4" />
                    </IconButton>
                  </Tooltip>
                )}
                <Tooltip title={t('shared.placeholders.matchCase')}>
                  <MatchCaseIcon
                    {...iconStyle}
                    aria-label={matchCase ? 'active' : 'inactive'}
                    onClick={handleToggleMatchCase}
                  />
                </Tooltip>
                <Tooltip title={t('shared.placeholders.matchWholeWord')}>
                  <MatchWholeWordIcon
                    {...iconStyle}
                    aria-label={matchWholeWord ? 'active' : 'inactive'}
                    onClick={handleToggleMatchWholeWord}
                  />
                </Tooltip>
                <Tooltip title={t('shared.placeholders.useRegex')}>
                  <UseRegularExpressionIcon
                    aria-label={useRegularExpression ? 'active' : 'inactive'}
                    {...iconStyle}
                    onClick={handleToggleUseRegularExpression}
                  />
                </Tooltip>
              </div>
            ),
          },
        }}
      />
    </Tooltip>
  )
}
