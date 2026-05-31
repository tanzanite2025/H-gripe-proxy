package config

import "github.com/tanzanite2025/mihomo-optimized/transport/kcptun"

type KcpTun struct {
	Enable        bool `json:"enable"`
	kcptun.Config `json:",inline"`
}
