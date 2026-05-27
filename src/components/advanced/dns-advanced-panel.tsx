/**
 * DNS 高级功能面板
 * 包含 DNS 统计、DNS 智能分流、DNS 零泄漏防护
 */

import { Box, Grid, Card, CardContent } from '@mui/material'
import { DnsStatsCard } from '@/components/setting/dns-stats-card'
import { DnsRoutingCard } from '@/components/setting/dns-routing-card'
import { DnsLeakProtectionCard } from '@/components/setting/dns-leak-protection-card'

export function DnsAdvancedPanel() {
  return (
    <Box>
      <Grid container spacing={2}>
        {/* DNS 统计 */}
        <Grid size={{ xs: 12, md: 6 }}>
          <DnsStatsCard />
        </Grid>

        {/* DNS 智能分流 + DNS 零泄漏防护 */}
        <Grid size={{ xs: 12, md: 6 }}>
          <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
            <Card>
              <CardContent>
                <DnsRoutingCard />
              </CardContent>
            </Card>
            
            <Card>
              <CardContent>
                <DnsLeakProtectionCard />
              </CardContent>
            </Card>
          </Box>
        </Grid>
      </Grid>
    </Box>
  )
}
