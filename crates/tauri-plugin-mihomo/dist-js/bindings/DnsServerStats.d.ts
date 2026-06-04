export type DnsServerStats = {
    server: string;
    queries: number;
    successes: number;
    failures: number;
    avgLatencyUs: number;
    lastQuery: string;
    lastError?: string | null;
};
