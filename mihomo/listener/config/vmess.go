package config

import (
	"encoding/json"

	"github.com/tanzanite2025/mihomo-optimized/listener/reality"
	"github.com/tanzanite2025/mihomo-optimized/listener/sing"
)

type VmessUser struct {
	Username string
	UUID     string
	AlterID  int
}

type VmessServer struct {
	Enable          bool
	Listen          string
	Users           []VmessUser
	WsPath          string
	GrpcServiceName string
	Certificate     string
	PrivateKey      string
	ClientAuthType  string
	ClientAuthCert  string
	EchKey          string
	RealityConfig   reality.Config
	MuxOption       sing.MuxOption `yaml:"mux-option" json:"mux-option,omitempty"`
}

func (t VmessServer) String() string {
	b, _ := json.Marshal(t)
	return string(b)
}
