/// 已知移动网络 ASN 列表

pub fn mobile_asns() -> &'static [(u32, &'static str)] {
    &[
        (9808, "China Mobile"), (57717, "China Mobile"), (58461, "China Mobile"),
        (24429, "China Mobile"), (134810, "China Mobile"),
        (10099, "China Unicom"), (9929, "China Unicom"), (4835, "China Unicom"),
        (4134, "China Telecom"), (4812, "China Telecom"), (4816, "China Telecom"),
        (3320, "Deutsche Telekom"), (5511, "Orange"), (2200, "Orange"),
        (1273, "Vodafone"), (3209, "Vodafone DE"),
        (7018, "AT&T Mobility"), (20057, "AT&T Mobility"),
        (21928, "T-Mobile US"),
        (7160, "SoftBank"), (2516, "KDDI"), (9605, "NTT Docomo"),
        (4766, "KT"), (4766, "SK Broadband"), (9318, "SK Broadband"),
        (9457, "LG U+"),
        (45899, "Viettel"), (7552, "Viettel"),
        (56040, "China Telecom Mobile"), (56041, "China Telecom Mobile"),
        (56042, "China Telecom Mobile"), (56043, "China Telecom Mobile"),
        (56044, "China Telecom Mobile"), (56045, "China Telecom Mobile"),
    ]
}

pub fn mobile_keywords() -> &'static [&'static str] {
    &[
        "mobile", "wireless", "cellular", "telecom", "telecommunication",
        "gsm", "lte", "5g", "4g", "3g",
        "china mobile", "china unicom", "china telecom",
        "vodafone", "orange", "t-mobile", "at&t", "verizon",
        "deutsche telekom", "telecom italia", "softbank", "kddi", "ntt docomo",
        "sk telecom", "kt", "lg u+", "viettel",
        "移动", "联通", "电信", "手机", "无线",
    ]
}
