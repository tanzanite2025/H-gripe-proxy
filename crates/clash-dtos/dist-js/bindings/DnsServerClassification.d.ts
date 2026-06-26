export type DnsServerClassification = {
    address: string;
    protocol: string;
    trustLevel: string;
    encrypted: boolean;
    description?: string | null;
};
