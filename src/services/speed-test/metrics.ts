const getAverage = (samples: number[]) =>
  samples.reduce((sum, value) => sum + value, 0) / samples.length

export function calculateStability(samples: number[]): number {
  if (samples.length < 2) {
    return 100
  }

  const average = getAverage(samples)
  if (average <= 0) {
    return 0
  }

  const variance =
    samples.reduce((sum, value) => sum + (value - average) ** 2, 0) /
    samples.length
  const standardDeviation = Math.sqrt(variance)
  const coefficientOfVariation = (standardDeviation / average) * 100

  return Math.round(Math.max(0, 100 - coefficientOfVariation))
}

export function calculateJitter(samples: number[]): number {
  if (samples.length < 2) {
    return 0
  }

  const average = getAverage(samples)
  const variance =
    samples.reduce((sum, value) => sum + (value - average) ** 2, 0) /
    samples.length
  const standardDeviation = Math.sqrt(variance)

  return Math.round(standardDeviation * 100) / 100
}
