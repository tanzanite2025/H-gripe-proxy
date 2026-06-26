import type { DnsServerClassification } from "./DnsServerClassification";

export type DnsTrustSummary = { total: number, encrypted: number, unencrypted: number, byTrustLevel: { [key in string]?: number }, servers: Array<DnsServerClassification>, leakRiskScore: number, lastEvaluated: string, };
