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
