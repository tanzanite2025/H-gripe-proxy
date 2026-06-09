use std::{fmt, str::FromStr};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClashMode {
    Rule,
    Global,
}

impl ClashMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Rule => "rule",
            Self::Global => "global",
        }
    }
}

impl FromStr for ClashMode {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "rule" => Ok(Self::Rule),
            "global" => Ok(Self::Global),
            mode => anyhow::bail!("invalid clash mode: {mode}"),
        }
    }
}

impl fmt::Display for ClashMode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_supported_modes_case_insensitively() {
        assert_eq!("rule".parse::<ClashMode>().unwrap(), ClashMode::Rule);
        assert_eq!("GLOBAL".parse::<ClashMode>().unwrap(), ClashMode::Global);
    }

    #[test]
    fn rejects_unsupported_modes() {
        assert!("script".parse::<ClashMode>().is_err());
        assert!("".parse::<ClashMode>().is_err());
        assert!(" Direct ".parse::<ClashMode>().is_err());
    }

    #[test]
    fn serializes_to_mihomo_mode_values() {
        assert_eq!(ClashMode::Rule.as_str(), "rule");
        assert_eq!(ClashMode::Global.as_str(), "global");
        assert_eq!(ClashMode::Global.to_string(), "global");
    }
}
