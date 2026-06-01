export type DnsCacheStats = {
    hit: number;
    miss: number;
    size: number;
    hitRate: number;
};
export type DnsQueryStats = {
    total: number;
    success: number;
    failed: number;
    avgLatencyUs: number;
    maxLatencyUs: number;
};
export type DnsServerStats = {
    server: string;
    queries: number;
    successes: number;
    failures: number;
    avgLatencyUs: number;
    lastQuery: string;
    lastError?: string;
};
export type DnsQueryEvent = {
    domain: string;
    qType: string;
    server: string;
    protocol: string;
    proxyName?: string | null;
    proxyChain?: string | null;
    egress?: string | null;
    rule?: string | null;
    rulePayload?: string | null;
    success: boolean;
    error?: string | null;
    latencyUs: number;
    timestamp: string;
};
export type DnsPollutedEntry = {
    domain: string;
    ip: string;
    timestamp: string;
    reason: string;
};
export type DnsPollutionStats = {
    totalChecked: number;
    pollutedCount: number;
    pollutionRate: number;
    recentPolluted: DnsPollutedEntry[];
};
export type DnsServerClassification = {
    address: string;
    protocol: string;
    trustLevel: string;
    encrypted: boolean;
    description?: string;
};
export type DnsTrustSummary = {
    total: number;
    encrypted: number;
    unencrypted: number;
    byTrustLevel: Record<string, number>;
    servers: DnsServerClassification[];
    leakRiskScore: number;
    lastEvaluated: string;
};
export type DnsMetrics = {
    cache: DnsCacheStats;
    queries: DnsQueryStats;
    servers: DnsServerStats[];
    recent: DnsQueryEvent[];
    pollution: DnsPollutionStats;
    trust: DnsTrustSummary;
};
