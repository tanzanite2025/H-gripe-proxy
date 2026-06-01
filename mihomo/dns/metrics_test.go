package dns

import (
	"testing"
	"time"
)

func TestDnsMetricsKeepsRecentQueryEvents(t *testing.T) {
	metrics := NewDnsMetricsForTest()

	for i := 0; i < recentQueryEventLimit+2; i++ {
		metrics.RecordQueryEvent(QueryEvent{
			Domain:    "example.org",
			QType:     "A",
			Server:    "https://dns.example/dns-query",
			Protocol:  "DoH",
			Success:   i%2 == 0,
			Error:     "boom",
			LatencyUs: uint64(i + 1),
			Timestamp: time.Unix(int64(i), 0).UTC(),
		})
	}

	recent := metrics.GetRecentQueries()
	if len(recent) != recentQueryEventLimit {
		t.Fatalf("expected %d recent query events, got %d", recentQueryEventLimit, len(recent))
	}
	if recent[0].LatencyUs != 3 {
		t.Fatalf("expected oldest retained event to have latency 3, got %d", recent[0].LatencyUs)
	}
	if recent[len(recent)-1].LatencyUs != uint64(recentQueryEventLimit+2) {
		t.Fatalf("expected newest event to be retained")
	}
	if recent[len(recent)-1].Timestamp == "" {
		t.Fatalf("expected timestamp to be serialized")
	}
}

func TestDnsMetricsKeepsRecentQueryProxyPath(t *testing.T) {
	metrics := NewDnsMetricsForTest()

	metrics.RecordQueryEvent(QueryEvent{
		Domain:      "example.org",
		QType:       "A",
		Server:      "https://dns.example/dns-query",
		Protocol:    "DoH",
		ProxyName:   "dns-out",
		ProxyChain:  "relay[dns-out]",
		Egress:      "203.0.113.10",
		Rule:        "DomainSuffix",
		RulePayload: "example.org",
		Success:     true,
		LatencyUs:   42,
		Timestamp:   time.Unix(1, 0).UTC(),
	})

	recent := metrics.GetRecentQueries()
	if len(recent) != 1 {
		t.Fatalf("expected one recent query event, got %d", len(recent))
	}
	if recent[0].ProxyName != "dns-out" {
		t.Fatalf("expected proxyName to be retained, got %q", recent[0].ProxyName)
	}
	if recent[0].Egress != "203.0.113.10" {
		t.Fatalf("expected egress to be retained, got %q", recent[0].Egress)
	}
	if recent[0].ProxyChain != "relay[dns-out]" {
		t.Fatalf("expected proxyChain to be retained, got %q", recent[0].ProxyChain)
	}
	if recent[0].Rule != "DomainSuffix" {
		t.Fatalf("expected rule to be retained, got %q", recent[0].Rule)
	}
	if recent[0].RulePayload != "example.org" {
		t.Fatalf("expected rulePayload to be retained, got %q", recent[0].RulePayload)
	}
}
