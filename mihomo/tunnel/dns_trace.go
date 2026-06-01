package tunnel

import (
	"context"

	C "github.com/tanzanite2025/mihomo-optimized/constant"
)

type dnsQueryTraceContextKey struct{}

type DNSQueryTrace struct {
	ProxyName   string
	ProxyChain  string
	Egress      string
	Rule        string
	RulePayload string
}

func WithDNSQueryTrace(ctx context.Context, trace *DNSQueryTrace) context.Context {
	return context.WithValue(ctx, dnsQueryTraceContextKey{}, trace)
}

func DNSQueryTraceFromContext(ctx context.Context) *DNSQueryTrace {
	trace, _ := ctx.Value(dnsQueryTraceContextKey{}).(*DNSQueryTrace)
	return trace
}

func recordDNSQueryTrace(ctx context.Context, rule C.Rule, proxyName string, remoteConn C.Connection) {
	trace := DNSQueryTraceFromContext(ctx)
	if trace == nil {
		return
	}

	if rule != nil {
		trace.Rule = rule.RuleType().String()
		trace.RulePayload = rule.Payload()
	}
	if proxyName != "" {
		trace.ProxyName = proxyName
	}
	if remoteConn != nil {
		if chain := remoteConn.Chains().String(); chain != "" {
			trace.ProxyChain = chain
		}
		if egress := remoteConn.RemoteDestination(); egress != "" {
			trace.Egress = egress
		}
	}
}
