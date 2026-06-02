package tunnel

import (
	"context"
	"testing"
	"time"
)

type egressMonitorTestRule struct{}

func (egressMonitorTestRule) RuleType() ruleTypeForEgressTest {
	return ruleTypeForEgressTest("DomainSuffix")
}

type ruleTypeForEgressTest string

func (r ruleTypeForEgressTest) String() string { return string(r) }

func TestEgressMonitorCurrentIPAndObservedCount(t *testing.T) {
	monitor := NewEgressMonitor()

	if got := monitor.CurrentIP(); got != "" {
		t.Fatalf("expected empty current IP before observations, got %q", got)
	}
	if got := monitor.ObservedCount(); got != 0 {
		t.Fatalf("expected zero observed IPs before observations, got %d", got)
	}

	monitor.RecordEgress("203.0.113.10")
	monitor.RecordEgress("203.0.113.10")

	if got := monitor.CurrentIP(); got != "203.0.113.10" {
		t.Fatalf("expected current IP to be retained, got %q", got)
	}
	if got := monitor.ObservedCount(); got != 1 {
		t.Fatalf("expected duplicate egress observations to be counted once, got %d", got)
	}

	monitor.RecordEgress("203.0.113.11")

	if got := monitor.CurrentIP(); got != "203.0.113.11" {
		t.Fatalf("expected latest egress IP, got %q", got)
	}
	if got := monitor.ObservedCount(); got != 2 {
		t.Fatalf("expected two unique egress IPs, got %d", got)
	}
}

func TestEgressMonitorRecordsIdentitySnapshot(t *testing.T) {
	monitor := NewEgressMonitor()

	monitor.RecordIdentity(EgressIdentityObservation{
		ProxyName:      "proxy-a",
		ProxyChain:     "relay[proxy-a]",
		ProxyEndpoint:  "203.0.113.10:443",
		PublicEgressIP: "198.51.100.20",
		Rule:           "DomainSuffix",
		RulePayload:    "example.org",
	})

	snapshot := monitor.Snapshot()

	if snapshot.PublicEgressIP != "198.51.100.20" {
		t.Fatalf("expected public egress IP to be retained, got %q", snapshot.PublicEgressIP)
	}
	if snapshot.ProxyEndpoint != "203.0.113.10:443" {
		t.Fatalf("expected proxy endpoint to be retained, got %q", snapshot.ProxyEndpoint)
	}
	if snapshot.ProxyName != "proxy-a" {
		t.Fatalf("expected proxy name to be retained, got %q", snapshot.ProxyName)
	}
	if snapshot.ProxyChain != "relay[proxy-a]" {
		t.Fatalf("expected proxy chain to be retained, got %q", snapshot.ProxyChain)
	}
	if snapshot.Rule != "DomainSuffix" {
		t.Fatalf("expected rule to be retained, got %q", snapshot.Rule)
	}
	if snapshot.RulePayload != "example.org" {
		t.Fatalf("expected rule payload to be retained, got %q", snapshot.RulePayload)
	}
	if snapshot.ObservedCount != 1 {
		t.Fatalf("expected observed count to include the egress IP, got %d", snapshot.ObservedCount)
	}
	if snapshot.ChangeCount != 0 {
		t.Fatalf("expected first identity observation not to count as an egress change, got %d", snapshot.ChangeCount)
	}
	if snapshot.UpdatedAt.IsZero() {
		t.Fatal("expected snapshot updated time to be set")
	}
}

func TestEgressMonitorDoesNotTreatProxyEndpointAsPublicEgress(t *testing.T) {
	monitor := NewEgressMonitor()

	monitor.RecordIdentity(EgressIdentityObservation{
		ProxyName:     "proxy-a",
		ProxyChain:    "proxy-a",
		ProxyEndpoint: "203.0.113.10:443",
	})

	snapshot := monitor.Snapshot()

	if snapshot.PublicEgressIP != "" {
		t.Fatalf("expected proxy endpoint not to be treated as public egress IP, got %q", snapshot.PublicEgressIP)
	}
	if snapshot.ProxyEndpoint != "203.0.113.10:443" {
		t.Fatalf("expected proxy endpoint to be retained, got %q", snapshot.ProxyEndpoint)
	}
	if snapshot.ObservedCount != 0 {
		t.Fatalf("expected proxy endpoint-only observation not to affect public egress stability, got %d", snapshot.ObservedCount)
	}
}

func TestEgressMonitorKeepsSnapshotsByProxyChain(t *testing.T) {
	monitor := NewEgressMonitor()

	monitor.RecordIdentity(EgressIdentityObservation{
		ProxyName:      "proxy-a",
		ProxyChain:     "relay[proxy-a]",
		PublicEgressIP: "198.51.100.20",
	})
	monitor.RecordIdentity(EgressIdentityObservation{
		ProxyName:      "proxy-b",
		ProxyChain:     "relay[proxy-b]",
		PublicEgressIP: "198.51.100.21",
	})

	snapshots := monitor.SnapshotsByProxyChain()

	if got := snapshots["relay[proxy-a]"].PublicEgressIP; got != "198.51.100.20" {
		t.Fatalf("expected proxy-a snapshot to be retained, got %q", got)
	}
	if got := snapshots["relay[proxy-b]"].PublicEgressIP; got != "198.51.100.21" {
		t.Fatalf("expected proxy-b snapshot to be retained, got %q", got)
	}
}

func TestEgressMonitorSchedulesPublicEgressProbeAndRecordsResult(t *testing.T) {
	monitor := NewEgressMonitor()
	probeCalls := 0

	err := monitor.MaybeProbePublicEgress(context.Background(), EgressProbeRequest{
		ProxyName:     "proxy-a",
		ProxyChain:    "relay[proxy-a]",
		ProxyEndpoint: "203.0.113.10:443",
		Rule:          "DomainSuffix",
		RulePayload:   "example.org",
		Probe: func(context.Context, EgressProbeRequest) (string, error) {
			probeCalls++
			return "198.51.100.20", nil
		},
		Now: func() time.Time {
			return time.Unix(1000, 0)
		},
	})
	if err != nil {
		t.Fatalf("expected probe to succeed, got %v", err)
	}

	snapshot := monitor.Snapshot()
	if probeCalls != 1 {
		t.Fatalf("expected one probe call, got %d", probeCalls)
	}
	if snapshot.PublicEgressIP != "198.51.100.20" {
		t.Fatalf("expected public egress IP from probe, got %q", snapshot.PublicEgressIP)
	}
	if snapshot.ProxyEndpoint != "203.0.113.10:443" {
		t.Fatalf("expected proxy endpoint to be retained, got %q", snapshot.ProxyEndpoint)
	}
	if snapshot.EgressSource != "publicProbe" {
		t.Fatalf("expected public probe source, got %q", snapshot.EgressSource)
	}
	if snapshot.Confidence != 90 {
		t.Fatalf("expected high confidence probe snapshot, got %d", snapshot.Confidence)
	}
	if snapshot.SampleCount != 1 {
		t.Fatalf("expected one sample, got %d", snapshot.SampleCount)
	}
	if snapshot.LastVerifiedAt.IsZero() {
		t.Fatal("expected last verified time to be set")
	}
}

func TestEgressMonitorRateLimitsPublicEgressProbePerProxyChain(t *testing.T) {
	monitor := NewEgressMonitor()
	probeCalls := 0
	now := time.Unix(1000, 0)
	request := EgressProbeRequest{
		ProxyName:     "proxy-a",
		ProxyChain:    "relay[proxy-a]",
		ProxyEndpoint: "203.0.113.10:443",
		Interval:      time.Minute,
		Probe: func(context.Context, EgressProbeRequest) (string, error) {
			probeCalls++
			return "198.51.100.20", nil
		},
		Now: func() time.Time {
			return now
		},
	}

	if err := monitor.MaybeProbePublicEgress(context.Background(), request); err != nil {
		t.Fatalf("expected first probe to succeed, got %v", err)
	}
	if err := monitor.MaybeProbePublicEgress(context.Background(), request); err != nil {
		t.Fatalf("expected rate-limited probe to be skipped without error, got %v", err)
	}

	if probeCalls != 1 {
		t.Fatalf("expected second probe inside interval to be skipped, got %d calls", probeCalls)
	}

	now = now.Add(time.Minute + time.Second)
	if err := monitor.MaybeProbePublicEgress(context.Background(), request); err != nil {
		t.Fatalf("expected probe after interval to succeed, got %v", err)
	}
	if probeCalls != 2 {
		t.Fatalf("expected probe after interval to run, got %d calls", probeCalls)
	}
}
