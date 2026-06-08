package adapter

import (
	"testing"

	"github.com/stretchr/testify/require"
	"github.com/tanzanite2025/mihomo-optimized/adapter/outbound"
	"github.com/tanzanite2025/mihomo-optimized/common/structure"
)

func TestDecodeHysteriaOptionAllowsSingleALPNString(t *testing.T) {
	decoder := structure.NewDecoder(structure.Option{
		TagName:          "proxy",
		WeaklyTypedInput: true,
		KeyReplacer:      structure.DefaultKeyReplacer,
	})

	option := &outbound.HysteriaOption{}
	err := decoder.Decode(map[string]any{
		"name":   "test-node",
		"type":   "hysteria",
		"server": "example.com",
		"port":   443,
		"up":     "10 Mbps",
		"down":   "10 Mbps",
		"alpn":   "hysteria",
	}, option)

	require.NoError(t, err)
	require.Equal(t, []string{"hysteria"}, option.ALPN)
}
