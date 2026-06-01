package tunnel

import (
	"context"
	"testing"

	C "github.com/tanzanite2025/mihomo-optimized/constant"
)

type dnsTraceTestRule struct{}

func (dnsTraceTestRule) RuleType() C.RuleType { return C.DomainSuffix }
func (dnsTraceTestRule) Match(*C.Metadata, C.RuleMatchHelper) (bool, string) {
	return true, "proxy-a"
}
func (dnsTraceTestRule) Adapter() string         { return "proxy-a" }
func (dnsTraceTestRule) Payload() string         { return "example.org" }
func (dnsTraceTestRule) ProviderNames() []string { return nil }

type dnsTraceTestConnection struct{}

func (dnsTraceTestConnection) Chains() C.Chain               { return C.Chain{"proxy-a", "relay"} }
func (dnsTraceTestConnection) ProviderChains() C.Chain       { return nil }
func (dnsTraceTestConnection) AppendToChains(C.ProxyAdapter) {}
func (dnsTraceTestConnection) RemoteDestination() string     { return "203.0.113.10:443" }

func TestDNSDialerTraceRecordsRuleAndProxyPath(t *testing.T) {
	trace := &DNSQueryTrace{}
	ctx := WithDNSQueryTrace(context.Background(), trace)

	recordDNSQueryTrace(ctx, dnsTraceTestRule{}, "proxy-a", dnsTraceTestConnection{})

	if trace.Rule != "DomainSuffix" {
		t.Fatalf("expected rule type to be recorded, got %q", trace.Rule)
	}
	if trace.RulePayload != "example.org" {
		t.Fatalf("expected rule payload to be recorded, got %q", trace.RulePayload)
	}
	if trace.ProxyName != "proxy-a" {
		t.Fatalf("expected proxy name to be recorded, got %q", trace.ProxyName)
	}
	if trace.ProxyChain != "relay[proxy-a]" {
		t.Fatalf("expected proxy chain to be recorded, got %q", trace.ProxyChain)
	}
	if trace.Egress != "203.0.113.10:443" {
		t.Fatalf("expected remote destination to be recorded, got %q", trace.Egress)
	}
}
