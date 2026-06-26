export type PerfStats = {
    goroutines: number;
    gogc: number;
    memLimit: number;
    heapAlloc: number;
    heapSys: number;
    heapInUse: number;
    stackInUse: number;
    numGc: number;
    gcPauseTotal: number;
    protectedConns: number;
    ruleVersion: string;
};
