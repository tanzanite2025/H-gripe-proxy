import type { SizeClassStats } from "./SizeClassStats";

export type BufferPoolStats = { totalAlloc: number, totalReturn: number, totalWaste: number, allocErrors: number, sizeClasses: Array<SizeClassStats>, };
