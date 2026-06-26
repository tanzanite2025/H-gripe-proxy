import type { DnsCacheStats } from "./DnsCacheStats";
import type { DnsPollutionStats } from "./DnsPollutionStats";
import type { DnsQueryEvent } from "./DnsQueryEvent";
import type { DnsQueryStats } from "./DnsQueryStats";
import type { DnsServerStats } from "./DnsServerStats";
import type { DnsTrustSummary } from "./DnsTrustSummary";

export type DnsMetrics = { cache: DnsCacheStats, queries: DnsQueryStats, servers: Array<DnsServerStats>, recent: Array<DnsQueryEvent>, pollution: DnsPollutionStats, trust: DnsTrustSummary, };
