package tunnel

import "testing"

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
		ProxyName:         "proxy-a",
		ProxyChain:        "relay[proxy-a]",
		RemoteDestination: "203.0.113.10:443",
		Rule:              "DomainSuffix",
		RulePayload:       "example.org",
	})

	snapshot := monitor.Snapshot()

	if snapshot.EgressIP != "203.0.113.10" {
		t.Fatalf("expected egress IP host to be extracted, got %q", snapshot.EgressIP)
	}
	if snapshot.ProxyName != "proxy-a" {
		t.Fatalf("expected proxy name to be retained, got %q", snapshot.ProxyName)
	}
	if snapshot.ProxyChain != "relay[proxy-a]" {
		t.Fatalf("expected proxy chain to be retained, got %q", snapshot.ProxyChain)
	}
	if snapshot.RemoteDestination != "203.0.113.10:443" {
		t.Fatalf("expected remote destination to be retained, got %q", snapshot.RemoteDestination)
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

func TestEgressMonitorRecordsHostOnlyRemoteDestination(t *testing.T) {
	monitor := NewEgressMonitor()

	monitor.RecordIdentity(EgressIdentityObservation{
		ProxyName:         "proxy-a",
		RemoteDestination: "198.51.100.20",
	})

	if got := monitor.Snapshot().EgressIP; got != "198.51.100.20" {
		t.Fatalf("expected host-only remote destination to be used as egress IP, got %q", got)
	}
}
