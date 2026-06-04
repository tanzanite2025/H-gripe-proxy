export type EgressStatus = {
    stable: boolean;
    changeCount: number;
    observedCount?: number;
    egressIp?: string;
    publicEgressIp?: string;
    proxyEndpoint?: string;
    proxyName?: string;
    proxyChain?: string;
    destinationAsn?: string;
    asnOrg?: string;
    rule?: string;
    rulePayload?: string;
    egressSource?: string;
    confidence?: number;
    sampleCount?: number;
    lastVerifiedAt?: string;
    updatedAt?: string;
};
