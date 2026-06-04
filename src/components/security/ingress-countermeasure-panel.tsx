import { ShieldCheck } from 'lucide-react'
import type { ChangeEvent } from 'react'

import { Card, Select, Stack, Switch, TextField } from '@/components/tailwind'
import type {
  ClassifierThresholds,
  DeceptionMode,
  EgressStabilitySupportConfig,
  IngressCountermeasureConfig,
  PersonaProfile,
  PersonaTone,
  SurfaceBias,
} from '@/services/coordinator'

interface Props {
  config: IngressCountermeasureConfig
  onChange: (config: IngressCountermeasureConfig) => void
}

const deceptionOptions: { value: DeceptionMode; label: string }[] = [
  { value: 'decoyPreferred', label: 'Decoy preferred' },
  { value: 'decoyOnly', label: 'Decoy only' },
  { value: 'observeOnly', label: 'Observe only' },
  { value: 'disabled', label: 'Disabled' },
]

const toneOptions: { value: PersonaTone; label: string }[] = [
  { value: 'restrained', label: 'Restrained' },
  { value: 'neutral', label: 'Neutral' },
  { value: 'helpful', label: 'Helpful' },
]

const surfaceBiasOptions: { value: SurfaceBias; label: string }[] = [
  { value: 'decoy', label: 'Decoy' },
  { value: 'balanced', label: 'Balanced' },
  { value: 'production', label: 'Production' },
]

function boundedNumber(value: string, fallback: number, min: number, max: number) {
  const parsed = Number.parseFloat(value)
  if (!Number.isFinite(parsed)) {
    return fallback
  }
  return Math.min(max, Math.max(min, parsed))
}

function boundedInteger(value: string, fallback: number, min: number) {
  const parsed = Number.parseInt(value, 10)
  if (!Number.isFinite(parsed)) {
    return fallback
  }
  return Math.max(min, parsed)
}

export function IngressCountermeasurePanel({ config, onChange }: Props) {
  const updateThresholds = (patch: Partial<ClassifierThresholds>) => {
    onChange({
      ...config,
      classifierThresholds: {
        ...config.classifierThresholds,
        ...patch,
      },
    })
  }

  const updateEgressSupport = (patch: Partial<EgressStabilitySupportConfig>) => {
    onChange({
      ...config,
      egressStabilitySupport: {
        ...config.egressStabilitySupport,
        ...patch,
      },
    })
  }

  const updatePersona = (
    id: string,
    updater: (profile: PersonaProfile) => PersonaProfile,
  ) => {
    onChange({
      ...config,
      personaProfiles: config.personaProfiles.map((profile) =>
        profile.id === id ? updater(profile) : profile,
      ),
    })
  }

  return (
    <Stack spacing={2}>
      <Card>
        <div className="p-4">
          <div className="mb-4 flex items-start justify-between gap-4">
            <div className="flex items-start gap-3">
              <ShieldCheck className="mt-0.5 h-5 w-5 shrink-0 text-primary" />
              <div>
                <div className="text-base font-semibold">Ingress countermeasure</div>
                <div className="text-xs text-muted-foreground">
                  Classifier-driven persona, deception, and egress stability controls.
                </div>
              </div>
            </div>
            <Switch
              checked={config.enabled}
              onCheckedChange={(enabled) => onChange({ ...config, enabled })}
            />
          </div>

          <div className="grid gap-3 md:grid-cols-3">
            <TextField
              label="Classifier low"
              type="number"
              value={String(config.classifierThresholds.lowConfidence)}
              onChange={(event: ChangeEvent<HTMLInputElement>) =>
                updateThresholds({
                  lowConfidence: boundedNumber(
                    event.target.value,
                    config.classifierThresholds.lowConfidence,
                    0,
                    1,
                  ),
                })
              }
              fullWidth
            />
            <TextField
              label="Classifier medium"
              type="number"
              value={String(config.classifierThresholds.mediumConfidence)}
              onChange={(event: ChangeEvent<HTMLInputElement>) =>
                updateThresholds({
                  mediumConfidence: boundedNumber(
                    event.target.value,
                    config.classifierThresholds.mediumConfidence,
                    0,
                    1,
                  ),
                })
              }
              fullWidth
            />
            <TextField
              label="Classifier high"
              type="number"
              value={String(config.classifierThresholds.highConfidence)}
              onChange={(event: ChangeEvent<HTMLInputElement>) =>
                updateThresholds({
                  highConfidence: boundedNumber(
                    event.target.value,
                    config.classifierThresholds.highConfidence,
                    0,
                    1,
                  ),
                })
              }
              fullWidth
            />
          </div>
        </div>
      </Card>

      <Card>
        <div className="p-4">
          <div className="mb-3 text-base font-semibold">Persona profiles</div>
          <div className="grid gap-3 md:grid-cols-2">
            {config.personaProfiles.map((profile) => (
              <div key={profile.id} className="rounded-lg border border-border p-3">
                <div className="mb-3">
                  <div className="text-sm font-semibold">{profile.label}</div>
                  <div className="text-xs text-muted-foreground">{profile.id}</div>
                </div>
                <div className="grid gap-3 sm:grid-cols-2">
                  <Select
                    label="Persona tone"
                    value={profile.tone}
                    options={toneOptions}
                    onChange={(value) =>
                      updatePersona(profile.id, (current) => ({
                        ...current,
                        tone: String(value) as PersonaTone,
                      }))
                    }
                  />
                  <Select
                    label="Surface bias"
                    value={profile.surfaceBias}
                    options={surfaceBiasOptions}
                    onChange={(value) =>
                      updatePersona(profile.id, (current) => ({
                        ...current,
                        surfaceBias: String(value) as SurfaceBias,
                      }))
                    }
                  />
                </div>
              </div>
            ))}
          </div>
        </div>
      </Card>

      <Card>
        <div className="p-4">
          <div className="mb-3 grid gap-3 md:grid-cols-2">
            <Select
              label="Deception mode"
              value={config.deceptionMode}
              options={deceptionOptions}
              onChange={(value) =>
                onChange({ ...config, deceptionMode: String(value) as DeceptionMode })
              }
            />
            <div className="flex items-center justify-between rounded-lg border border-border p-3">
              <div>
                <div className="text-sm font-semibold">Egress stability support</div>
                <div className="text-xs text-muted-foreground">
                  Reduce drift while suspicious or hostile ingress flows are active.
                </div>
              </div>
              <Switch
                checked={config.egressStabilitySupport.enabled}
                onCheckedChange={(enabled) => updateEgressSupport({ enabled })}
              />
            </div>
          </div>

          <div className="grid gap-3 md:grid-cols-4">
            <TextField
              label="Soft delay min"
              type="number"
              value={String(config.responseDelayRanges.softDelayMinMs)}
              onChange={(event: ChangeEvent<HTMLInputElement>) =>
                onChange({
                  ...config,
                  responseDelayRanges: {
                    ...config.responseDelayRanges,
                    softDelayMinMs: boundedInteger(
                      event.target.value,
                      config.responseDelayRanges.softDelayMinMs,
                      0,
                    ),
                  },
                })
              }
              fullWidth
            />
            <TextField
              label="Soft delay max"
              type="number"
              value={String(config.responseDelayRanges.softDelayMaxMs)}
              onChange={(event: ChangeEvent<HTMLInputElement>) =>
                onChange({
                  ...config,
                  responseDelayRanges: {
                    ...config.responseDelayRanges,
                    softDelayMaxMs: boundedInteger(
                      event.target.value,
                      config.responseDelayRanges.softDelayMaxMs,
                      0,
                    ),
                  },
                })
              }
              fullWidth
            />
            <TextField
              label="Hard delay min"
              type="number"
              value={String(config.responseDelayRanges.hardDelayMinMs)}
              onChange={(event: ChangeEvent<HTMLInputElement>) =>
                onChange({
                  ...config,
                  responseDelayRanges: {
                    ...config.responseDelayRanges,
                    hardDelayMinMs: boundedInteger(
                      event.target.value,
                      config.responseDelayRanges.hardDelayMinMs,
                      0,
                    ),
                  },
                })
              }
              fullWidth
            />
            <TextField
              label="Hard delay max"
              type="number"
              value={String(config.responseDelayRanges.hardDelayMaxMs)}
              onChange={(event: ChangeEvent<HTMLInputElement>) =>
                onChange({
                  ...config,
                  responseDelayRanges: {
                    ...config.responseDelayRanges,
                    hardDelayMaxMs: boundedInteger(
                      event.target.value,
                      config.responseDelayRanges.hardDelayMaxMs,
                      0,
                    ),
                  },
                })
              }
              fullWidth
            />
          </div>
        </div>
      </Card>
    </Stack>
  )
}
