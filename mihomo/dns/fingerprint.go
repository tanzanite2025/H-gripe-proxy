package dns

import (
	"crypto/rand"
	"math/big"

	icontext "github.com/metacubex/mihomo/context"
	"github.com/metacubex/mihomo/log"

	D "github.com/miekg/dns"
)

// FingerprintProtection applies DNS fingerprint countermeasures to outgoing queries:
// - Randomize query ID (already done by miekg/dns, but we ensure it)
// - Add EDNS0 padding option to increase query size uniformity
// - Randomize EDNS0 UDP size within a range
// This makes DNS queries harder to fingerprint by network observers.

const (
	// EDNS0 padding block size per RFC 7830/RFC 8467
	paddingBlockSize = 128
	// Min/max EDNS0 UDP size to randomize
	minUDPSize = 512
	maxUDPSize = 4096
)

// randomUint16 generates a cryptographically random uint16.
func randomUint16() uint16 {
	n, err := rand.Int(rand.Reader, big.NewInt(65536))
	if err != nil {
		// Fallback to less random but functional
		return uint16(n.Uint64())
	}
	return uint16(n.Uint64())
}

// randomUDPSize generates a random EDNS0 UDP buffer size.
func randomUDPSize() uint16 {
	rangeSize := maxUDPSize - minUDPSize
	n, err := rand.Int(rand.Reader, big.NewInt(int64(rangeSize)))
	if err != nil {
		return 4096 // safe default
	}
	return uint16(minUDPSize + n.Uint64())
}

// addPadding adds EDNS0 padding to a DNS message per RFC 8467.
// Padding length is chosen to bring the total message size to a multiple of paddingBlockSize.
func addPadding(msg *D.Msg) {
	if msg == nil {
		return
	}

	// Find or create OPT record
	var opt *D.OPT
	for _, rr := range msg.Extra {
		if o, ok := rr.(*D.OPT); ok {
			opt = o
			break
		}
	}
	if opt == nil {
		opt = &D.OPT{
			Hdr: D.RR_Header{
				Name:   ".",
				Rrtype: D.TypeOPT,
			},
		}
		msg.Extra = append(msg.Extra, opt)
	}

	// Calculate padding length
	// We want the total message to be a multiple of paddingBlockSize
	msgLen := msg.Len()
	paddingLen := paddingBlockSize - (msgLen % paddingBlockSize)
	if paddingLen < 4 {
		// Minimum padding option is 4 bytes (1 byte type + 1 byte length + 0-2 bytes padding)
		paddingLen += paddingBlockSize
	}

	// Padding option: code 12 per RFC 7830
	padOpt := &D.EDNS0_PADDING{
		Padding: make([]byte, paddingLen-4), // -4 for EDNS0 option header
	}
	opt.Option = append(opt.Option, padOpt)
}

// applyFingerprintProtection modifies an outgoing DNS query to resist fingerprinting.
func applyFingerprintProtection(msg *D.Msg) {
	if msg == nil {
		return
	}

	// Ensure random query ID
	msg.Id = randomUint16()

	// Randomize EDNS0 UDP size
	for _, rr := range msg.Extra {
		if opt, ok := rr.(*D.OPT); ok {
			opt.SetUDPSize(randomUDPSize())
			break
		}
	}

	// Add padding for size uniformity
	addPadding(msg)

	log.Debugln("[DNS] fingerprint protection applied to query for %s (id=%d)", msgToDomain(msg), msg.Id)
}

// withFingerprintProtection is a middleware that applies DNS fingerprint protection
// to outgoing queries before they are sent to upstream resolvers.
func withFingerprintProtection(enabled bool) middleware {
	if !enabled {
		return func(next handler) handler {
			return next
		}
	}

	return func(next handler) handler {
		return func(ctx *icontext.DNSContext, r *D.Msg) (*D.Msg, error) {
			// Apply fingerprint protection to the outgoing query
			applyFingerprintProtection(r)

			return next(ctx, r)
		}
	}
}
