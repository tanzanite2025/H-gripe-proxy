package congestion

import (
	"sync/atomic"
	"time"

	"github.com/metacubex/quic-go/congestion"
	"github.com/metacubex/quic-go/monotime"
)

const (
	// Adaptive probing parameters
	probeIntervalSec = 10 // probe every 10s
	probeStepUp      = 1.2
	probeStepDown    = 0.8
	minAckRateProbe  = 0.85
	maxAckRateProbe  = 0.95
	maxBpsMultiplier = 4
	minBpsDivisor    = 4 // base / 4 = 0.25x
)

// AdaptiveBrutalSender extends BrutalSender with adaptive bandwidth probing.
// It periodically probes for higher bandwidth when the network is underutilized,
// and backs off when packet loss increases.
type AdaptiveBrutalSender struct {
	rttStats        congestion.RTTStatsProvider
	maxDatagramSize congestion.ByteCount
	pacer           *pacer

	// Base bandwidth (configured)
	baseBps congestion.ByteCount
	// Current effective bandwidth (atomic for lock-free reads)
	currentBps atomic.Int64
	// Maximum bandwidth (probing ceiling)
	maxBps congestion.ByteCount
	// Minimum bandwidth (probing floor)
	minBps congestion.ByteCount

	// Probing state
	lastProbeTimestamp int64 // monotime slot timestamp

	// Packet info for adaptive decisions (same slot structure as BrutalSender)
	pktInfoSlots [pktInfoSlotCount]pktInfo
	ackRate      float64
}

var _ congestion.CongestionControlEx = &AdaptiveBrutalSender{}

// NewAdaptiveBrutalSender creates a bandwidth-adaptive congestion controller.
func NewAdaptiveBrutalSender(baseBps congestion.ByteCount) *AdaptiveBrutalSender {
	abs := &AdaptiveBrutalSender{
		baseBps:         baseBps,
		maxBps:          baseBps * maxBpsMultiplier,
		minBps:          baseBps / minBpsDivisor,
		maxDatagramSize: initMaxDatagramSize,
		ackRate:         1,
	}
	abs.currentBps.Store(int64(baseBps))
	abs.pacer = newPacer(func() congestion.ByteCount {
		return congestion.ByteCount(float64(abs.currentBps.Load()) / abs.ackRate)
	})
	return abs
}

func (b *AdaptiveBrutalSender) SetRTTStatsProvider(rttStats congestion.RTTStatsProvider) {
	b.rttStats = rttStats
}

func (b *AdaptiveBrutalSender) TimeUntilSend(bytesInFlight congestion.ByteCount) monotime.Time {
	return b.pacer.TimeUntilSend()
}

func (b *AdaptiveBrutalSender) HasPacingBudget(now monotime.Time) bool {
	return b.pacer.Budget(now) >= b.maxDatagramSize
}

func (b *AdaptiveBrutalSender) CanSend(bytesInFlight congestion.ByteCount) bool {
	return bytesInFlight < b.GetCongestionWindow()
}

func (b *AdaptiveBrutalSender) GetCongestionWindow() congestion.ByteCount {
	rtt := maxDuration(b.rttStats.LatestRTT(), b.rttStats.SmoothedRTT())
	if rtt <= 0 {
		return 10240
	}
	return congestion.ByteCount(float64(b.currentBps.Load()) * rtt.Seconds() * 1.5 / b.ackRate)
}

func (b *AdaptiveBrutalSender) OnPacketSent(sentTime monotime.Time, bytesInFlight congestion.ByteCount,
	packetNumber congestion.PacketNumber, bytes congestion.ByteCount, isRetransmittable bool) {
	b.pacer.SentPacket(sentTime, bytes)
}

func (b *AdaptiveBrutalSender) OnPacketAcked(number congestion.PacketNumber, ackedBytes congestion.ByteCount,
	priorInFlight congestion.ByteCount, eventTime monotime.Time) {
	// Stub
}

func (b *AdaptiveBrutalSender) OnCongestionEvent(number congestion.PacketNumber, lostBytes congestion.ByteCount,
	priorInFlight congestion.ByteCount) {
	// Immediate backoff on loss
	b.maybeBackOff()
}

func (b *AdaptiveBrutalSender) OnCongestionEventEx(priorInFlight congestion.ByteCount, eventTime monotime.Time,
	ackedPackets []congestion.AckedPacketInfo, lostPackets []congestion.LostPacketInfo) {
	currentTimestamp := int64(time.Duration(eventTime) / time.Second)
	slot := currentTimestamp % pktInfoSlotCount
	if b.pktInfoSlots[slot].Timestamp == currentTimestamp {
		b.pktInfoSlots[slot].LossCount += uint64(len(lostPackets))
		b.pktInfoSlots[slot].AckCount += uint64(len(ackedPackets))
	} else {
		b.pktInfoSlots[slot].Timestamp = currentTimestamp
		b.pktInfoSlots[slot].AckCount = uint64(len(ackedPackets))
		b.pktInfoSlots[slot].LossCount = uint64(len(lostPackets))
	}
	b.updateAckRate(currentTimestamp)

	// Immediate backoff if significant loss
	if len(lostPackets) > 0 {
		b.maybeBackOff()
	}
}

func (b *AdaptiveBrutalSender) SetMaxDatagramSize(size congestion.ByteCount) {
	b.maxDatagramSize = size
	b.pacer.SetMaxDatagramSize(size)
}

func (b *AdaptiveBrutalSender) InSlowStart() bool {
	return false
}

func (b *AdaptiveBrutalSender) InRecovery() bool {
	return false
}

func (b *AdaptiveBrutalSender) MaybeExitSlowStart() {}

func (b *AdaptiveBrutalSender) OnRetransmissionTimeout(packetsRetransmitted bool) {}

// updateAckRate updates the ack rate and triggers adaptive probing.
func (b *AdaptiveBrutalSender) updateAckRate(currentTimestamp int64) {
	minTimestamp := currentTimestamp - pktInfoSlotCount
	var ackCount, lossCount uint64
	for _, info := range b.pktInfoSlots {
		if info.Timestamp < minTimestamp {
			continue
		}
		ackCount += info.AckCount
		lossCount += info.LossCount
	}
	if ackCount+lossCount < minSampleCount {
		b.ackRate = 1
		return
	}
	rate := float64(ackCount) / float64(ackCount+lossCount)
	if rate < minAckRate {
		b.ackRate = minAckRate
	} else {
		b.ackRate = rate
	}

	// Periodic adaptive probing
	if currentTimestamp-b.lastProbeTimestamp >= probeIntervalSec {
		b.lastProbeTimestamp = currentTimestamp
		b.adaptiveProbe()
	}
}

// adaptiveProbe adjusts bandwidth based on current network health.
func (b *AdaptiveBrutalSender) adaptiveProbe() {
	currentBps := congestion.ByteCount(b.currentBps.Load())

	if b.ackRate >= maxAckRateProbe && currentBps < b.maxBps {
		// Network is healthy, try higher bandwidth
		newBps := congestion.ByteCount(float64(currentBps) * probeStepUp)
		if newBps > b.maxBps {
			newBps = b.maxBps
		}
		b.currentBps.Store(int64(newBps))
	} else if b.ackRate < minAckRateProbe && currentBps > b.minBps {
		// Network is struggling, reduce bandwidth
		newBps := congestion.ByteCount(float64(currentBps) * probeStepDown)
		if newBps < b.minBps {
			newBps = b.minBps
		}
		b.currentBps.Store(int64(newBps))
	}
}

// maybeBackOff immediately reduces bandwidth on packet loss.
func (b *AdaptiveBrutalSender) maybeBackOff() {
	currentBps := congestion.ByteCount(b.currentBps.Load())
	if currentBps <= b.minBps {
		return
	}
	newBps := congestion.ByteCount(float64(currentBps) * probeStepDown)
	if newBps < b.minBps {
		newBps = b.minBps
	}
	b.currentBps.Store(int64(newBps))
}

// GetCurrentBps returns the current effective bandwidth.
func (b *AdaptiveBrutalSender) GetCurrentBps() congestion.ByteCount {
	return congestion.ByteCount(b.currentBps.Load())
}

// GetBaseBps returns the configured base bandwidth.
func (b *AdaptiveBrutalSender) GetBaseBps() congestion.ByteCount {
	return b.baseBps
}
