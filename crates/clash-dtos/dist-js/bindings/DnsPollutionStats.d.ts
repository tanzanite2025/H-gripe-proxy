import type { DnsPollutedEntry } from "./DnsPollutedEntry";
export type DnsPollutionStats = {
    totalChecked: number;
    pollutedCount: number;
    pollutionRate: number;
    recentPolluted: Array<DnsPollutedEntry>;
};
