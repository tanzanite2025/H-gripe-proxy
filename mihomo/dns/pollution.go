package dns

import (
	"net/netip"
	"sync"
	"sync/atomic"
	"time"

	icontext "github.com/tanzanite2025/mihomo-optimized/context"
	"github.com/tanzanite2025/mihomo-optimized/log"

	D "github.com/miekg/dns"
)

// PollutionDetector detects DNS pollution by checking responses against
// known polluted IP ranges and heuristics.
type PollutionDetector struct {
	// Known polluted IP prefixes (commonly used by GFW and ISP hijacking)
	pollutedPrefixes []netip.Prefix

	// Statistics
	totalChecked  atomic.Uint64
	pollutedCount atomic.Uint64

	// Recent polluted domains (ring buffer)
	mu             sync.Mutex
	recentPolluted []pollutedEntry
	maxRecent      int
}

type pollutedEntry struct {
	Domain    string    `json:"domain"`
	IP        string    `json:"ip"`
	Timestamp time.Time `json:"timestamp"`
	Reason    string    `json:"reason"`
}

// PollutionStats returns pollution detection statistics.
type PollutionStats struct {
	TotalChecked  uint64           `json:"totalChecked"`
	PollutedCount uint64           `json:"pollutedCount"`
	PollutionRate float64          `json:"pollutionRate"`
	RecentPolluted []pollutedEntry `json:"recentPolluted"`
}

var defaultPollutionDetector = &PollutionDetector{
	pollutedPrefixes: []netip.Prefix{
		// Common GFW pollution IPs
		parsePrefix("93.46.8.0/24"),   // Known GFW pollution
		parsePrefix("37.61.54.0/24"),   // Known GFW pollution
		parsePrefix("8.7.198.0/24"),    // Known GFW pollution
		parsePrefix("46.82.174.0/24"),  // Known GFW pollution
		parsePrefix("78.16.0.0/24"),    // Known GFW pollution
		parsePrefix("59.24.3.0/24"),    // Known GFW pollution
		parsePrefix("243.185.187.0/24"), // Known GFW pollution
		parsePrefix("203.98.7.0/24"),   // Known GFW pollution
		parsePrefix("159.106.121.0/24"), // Known GFW pollution
		parsePrefix("69.55.52.0/24"),   // Known GFW pollution
		parsePrefix("69.171.138.0/24"), // Known GFW pollution
		parsePrefix("185.70.196.0/24"), // Known GFW pollution
		// ISP hijacking ranges (common landing pages)
		parsePrefix("1.1.127.0/24"),    // China Telecom hijack
		parsePrefix("1.2.4.0/24"),      // China Telecom DNS redirect
		parsePrefix("36.251.136.0/24"), // China Unicom hijack
		parsePrefix("110.43.0.0/16"),   // China Mobile hijack
		parsePrefix("123.129.254.0/24"), // DNS hijack landing
		parsePrefix("183.232.0.0/16"),  // China Mobile DNS redirect
		// Bogon/reserved ranges that should never appear in DNS answers
		parsePrefix("0.0.0.0/8"),       // Current network
		parsePrefix("100.64.0.0/10"),   // Shared address space (CGNAT)
		parsePrefix("127.0.0.0/8"),     // Loopback
		parsePrefix("224.0.0.0/4"),     // Multicast
		parsePrefix("240.0.0.0/4"),     // Reserved
	},
	maxRecent: 50,
}

func parsePrefix(s string) netip.Prefix {
	p, err := netip.ParsePrefix(s)
	if err != nil {
		log.Warnln("[DNS] failed to parse polluted prefix %s: %v", s, err)
		return netip.Prefix{}
	}
	return p
}

// IsPolluted checks if an IP address is likely a polluted DNS response.
func (d *PollutionDetector) IsPolluted(ip netip.Addr) (bool, string) {
	for _, prefix := range d.pollutedPrefixes {
		if prefix.IsValid() && prefix.Contains(ip) {
			return true, "known_polluted_range"
		}
	}

	if !ip.Is4() {
		return false, ""
	}

	// Heuristic: single-digit first octet with non-standard patterns
	// (many pollution responses use unusual IPs)
	b := ip.As4()
	if b[0] == 0 && (b[1] != 0 || b[2] != 0 || b[3] != 0) {
		return true, "zero_network"
	}

	return false, ""
}

// CheckMsg checks a DNS response message for pollution indicators.
// Returns the list of polluted IPs found in the answer section.
func (d *PollutionDetector) CheckMsg(msg *D.Msg) []netip.Addr {
	if msg == nil || len(msg.Answer) == 0 {
		return nil
	}

	d.totalChecked.Add(1)

	var polluted []netip.Addr
	for _, rr := range msg.Answer {
		var ip netip.Addr
		switch a := rr.(type) {
		case *D.A:
			addr, ok := netip.AddrFromSlice(a.A)
			if ok {
				ip = addr
			}
		case *D.AAAA:
			addr, ok := netip.AddrFromSlice(a.AAAA)
			if ok {
				ip = addr
			}
		default:
			continue
		}

		if !ip.IsValid() {
			continue
		}

		if isPolluted, reason := d.IsPolluted(ip); isPolluted {
			polluted = append(polluted, ip)
			d.pollutedCount.Add(1)

			domain := msgToDomain(msg)
			d.mu.Lock()
			d.recentPolluted = append(d.recentPolluted, pollutedEntry{
				Domain:    domain,
				IP:        ip.String(),
				Timestamp: time.Now(),
				Reason:    reason,
			})
			if len(d.recentPolluted) > d.maxRecent {
				d.recentPolluted = d.recentPolluted[d.maxRecent:]
			}
			d.mu.Unlock()

			log.Debugln("[DNS] pollution detected: %s -> %s (%s)", domain, ip.String(), reason)
		}
	}

	return polluted
}

// GetStats returns current pollution detection statistics.
func (d *PollutionDetector) GetStats() PollutionStats {
	total := d.totalChecked.Load()
	polluted := d.pollutedCount.Load()
	var rate float64
	if total > 0 {
		rate = float64(polluted) / float64(total)
	}

	d.mu.Lock()
	recent := make([]pollutedEntry, len(d.recentPolluted))
	copy(recent, d.recentPolluted)
	d.mu.Unlock()

	return PollutionStats{
		TotalChecked:   total,
		PollutedCount:  polluted,
		PollutionRate:  rate,
		RecentPolluted: recent,
	}
}

// GetPollutionDetector returns the global pollution detector instance.
func GetPollutionDetector() *PollutionDetector {
	return defaultPollutionDetector
}

// withPollutionDetection is a middleware that checks DNS responses for pollution.
func withPollutionDetection() middleware {
	return func(next handler) handler {
		return func(ctx *icontext.DNSContext, r *D.Msg) (*D.Msg, error) {
			msg, err := next(ctx, r)
			if err != nil || msg == nil {
				return msg, err
			}

			// Only check responses that have answers
			if len(msg.Answer) > 0 {
				polluted := GetPollutionDetector().CheckMsg(msg)
				if len(polluted) > 0 {
					// Mark context with pollution detection result
					ctx.SetType(icontext.DNSTypeRaw)
					log.Warnln("[DNS] response for %s contains %d polluted IP(s), first: %s",
						msgToDomain(msg), len(polluted), polluted[0].String())
				}
			}

			return msg, nil
		}
	}
}
