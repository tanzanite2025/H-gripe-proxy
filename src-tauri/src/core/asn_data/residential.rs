/// Known fixed-line residential broadband ASN entries.

pub fn residential_asns() -> &'static [(u32, &'static str)] {
    &[
        (7922, "Comcast Cable"),
        (7018, "AT&T Internet"),
        (22773, "Cox Communications"),
        (20115, "Charter Communications"),
        (20001, "Charter Communications"),
        (11427, "Charter Communications"),
        (5650, "Frontier/Ziply"),
        (6128, "Cablevision/Optimum"),
        (6128, "Altice USA"),
        (701, "Verizon Fios"),
        (30036, "Mediacom Communications"),
        (33751, "Wave Broadband"),
        (22724, "Astound Broadband"),
        (46375, "Google Fiber"),
        (16591, "Google Fiber"),
        (18403, "FPT Telecom"),
        (4766, "Korea Telecom Broadband"),
        (9318, "SK Broadband"),
    ]
}

pub fn residential_keywords() -> &'static [&'static str] {
    &[
        "broadband",
        "cable",
        "fiber",
        "fibre",
        "ftth",
        "fios",
        "dsl",
        "xfinity",
        "comcast",
        "charter",
        "spectrum",
        "cox",
        "frontier",
        "ziply",
        "optimum",
        "altice",
        "verizon fios",
        "google fiber",
        "mediacom",
        "wave broadband",
        "astound broadband",
        "telecom residential",
    ]
}
