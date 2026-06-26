export type TLSFingerprintStats = {
    currentFingerprint: string;
    rotationCount: number;
    usageSnapshot: {
        [key in string]?: number;
    };
};
