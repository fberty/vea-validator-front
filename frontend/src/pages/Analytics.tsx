import { useState } from 'react'
import { Layout } from '../components/Layout'
import { Card } from '../components/Card'
import { Button } from '../components/Button'
import { Badge } from '../components/Badge'
import {
  BarChart3,
  TrendingUp,
  TrendingDown,
  Download,
  Calendar,
  Activity,
  Clock,
  CheckCircle2,
  XCircle,
  Filter
} from 'lucide-react'
import {
  LineChart,
  Line,
  AreaChart,
  Area,
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  PieChart,
  Pie,
  Cell
} from 'recharts'

const timeRanges = [
  { id: 'day', label: '24h' },
  { id: 'week', label: '7d' },
  { id: 'month', label: '30d' },
  { id: 'all', label: 'All' },
]

const performanceData = [
  { time: '00:00', success: 12, failed: 0, latency: 1.2 },
  { time: '04:00', success: 8, failed: 1, latency: 1.5 },
  { time: '08:00', success: 15, failed: 0, latency: 1.1 },
  { time: '12:00', success: 22, failed: 0, latency: 1.3 },
  { time: '16:00', success: 18, failed: 1, latency: 1.8 },
  { time: '20:00', success: 14, failed: 0, latency: 1.2 },
  { time: '24:00', success: 10, failed: 0, latency: 1.4 },
]

const volumeData = [
  { name: 'Mon', arb_eth: 45, arb_gnosis: 32 },
  { name: 'Tue', arb_eth: 52, arb_gnosis: 28 },
  { name: 'Wed', arb_eth: 38, arb_gnosis: 41 },
  { name: 'Thu', arb_eth: 61, arb_gnosis: 35 },
  { name: 'Fri', arb_eth: 55, arb_gnosis: 48 },
  { name: 'Sat', arb_eth: 42, arb_gnosis: 29 },
  { name: 'Sun', arb_eth: 35, arb_gnosis: 24 },
]

const taskDistribution = [
  { name: 'SaveSnapshot', value: 35, color: '#6366f1' },
  { name: 'ValidateClaim', value: 25, color: '#8b5cf6' },
  { name: 'StartVerification', value: 15, color: '#a855f7' },
  { name: 'VerifySnapshot', value: 12, color: '#d946ef' },
  { name: 'WithdrawDeposit', value: 8, color: '#ec4899' },
  { name: 'Other', value: 5, color: '#64748b' },
]

const recentMetrics = [
  { label: 'Total Tasks', value: '1,247', change: '+12%', positive: true },
  { label: 'Success Rate', value: '99.8%', change: '+0.2%', positive: true },
  { label: 'Avg Latency', value: '1.3s', change: '-0.1s', positive: true },
  { label: 'Challenges', value: '2', change: '+1', positive: false },
]

const errorLog = [
  { timestamp: '2024-01-15 14:23', type: 'RootNotConfirmed', route: 'ARB_TO_ETH', epoch: 1244 },
  { timestamp: '2024-01-14 09:15', type: 'InsufficientFunds', route: 'ARB_TO_GNOSIS', epoch: 1241 },
  { timestamp: '2024-01-12 18:45', type: 'RootNotConfirmed', route: 'ARB_TO_ETH', epoch: 1238 },
]

export function Analytics() {
  const [timeRange, setTimeRange] = useState('week')
  const [selectedRoute, setSelectedRoute] = useState('all')

  const handleExport = (format: 'csv' | 'json') => {
    const data = {
      timeRange,
      route: selectedRoute,
      exportedAt: new Date().toISOString(),
      metrics: recentMetrics,
      performance: performanceData,
      volume: volumeData,
    }

    const content = format === 'json' 
      ? JSON.stringify(data, null, 2)
      : 'timestamp,success,failed,latency\n' + performanceData.map(d => 
          `${d.time},${d.success},${d.failed},${d.latency}`
        ).join('\n')

    const blob = new Blob([content], { type: format === 'json' ? 'application/json' : 'text/csv' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `vea-analytics-${timeRange}.${format}`
    a.click()
    URL.revokeObjectURL(url)
  }

  return (
    <Layout>
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {/* Header */}
        <div className="flex flex-col lg:flex-row lg:items-center lg:justify-between gap-4 mb-8">
          <div>
            <h1 className="text-2xl font-bold">Analytics</h1>
            <p className="text-[var(--color-text-muted)] mt-1">Historical metrics and performance data</p>
          </div>
          <div className="flex flex-wrap items-center gap-3">
            {/* Time Range Selector */}
            <div className="flex bg-[var(--color-surface)] border border-[var(--color-border)] rounded-lg p-1">
              {timeRanges.map((range) => (
                <button
                  key={range.id}
                  onClick={() => setTimeRange(range.id)}
                  className={`px-3 py-1.5 text-sm font-medium rounded-md transition-colors ${
                    timeRange === range.id
                      ? 'bg-[var(--color-primary)] text-white'
                      : 'text-[var(--color-text-muted)] hover:text-white'
                  }`}
                >
                  {range.label}
                </button>
              ))}
            </div>

            {/* Route Filter */}
            <div className="flex items-center gap-2">
              <Filter size={16} className="text-[var(--color-text-muted)]" />
              <select
                value={selectedRoute}
                onChange={(e) => setSelectedRoute(e.target.value)}
                className="bg-[var(--color-surface)] border border-[var(--color-border)] rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-[var(--color-primary)]"
              >
                <option value="all">All Routes</option>
                <option value="arb_eth">Arbitrum → Ethereum</option>
                <option value="arb_gnosis">Arbitrum → Gnosis</option>
              </select>
            </div>

            {/* Export Buttons */}
            <div className="flex gap-2">
              <Button variant="secondary" size="sm" icon={<Download size={14} />} onClick={() => handleExport('csv')}>
                CSV
              </Button>
              <Button variant="secondary" size="sm" icon={<Download size={14} />} onClick={() => handleExport('json')}>
                JSON
              </Button>
            </div>
          </div>
        </div>

        {/* Summary Cards */}
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-8">
          {recentMetrics.map((metric, i) => (
            <Card key={i}>
              <p className="text-sm text-[var(--color-text-muted)]">{metric.label}</p>
              <div className="flex items-end justify-between mt-2">
                <p className="text-2xl font-bold">{metric.value}</p>
                <div className={`flex items-center gap-1 text-sm ${
                  metric.positive ? 'text-emerald-400' : 'text-red-400'
                }`}>
                  {metric.positive ? <TrendingUp size={14} /> : <TrendingDown size={14} />}
                  {metric.change}
                </div>
              </div>
            </Card>
          ))}
        </div>

        {/* Charts Grid */}
        <div className="grid lg:grid-cols-2 gap-8 mb-8">
          {/* Performance Chart */}
          <Card>
            <div className="flex items-center justify-between mb-6">
              <div>
                <h3 className="font-semibold">Task Performance</h3>
                <p className="text-sm text-[var(--color-text-muted)]">Success vs failed tasks over time</p>
              </div>
              <Badge variant="success">
                <Activity size={12} className="mr-1" />
                Live
              </Badge>
            </div>
            <div className="h-64">
              <ResponsiveContainer width="100%" height="100%">
                <AreaChart data={performanceData}>
                  <defs>
                    <linearGradient id="successGradient" x1="0" y1="0" x2="0" y2="1">
                      <stop offset="5%" stopColor="#10b981" stopOpacity={0.3} />
                      <stop offset="95%" stopColor="#10b981" stopOpacity={0} />
                    </linearGradient>
                  </defs>
                  <CartesianGrid strokeDasharray="3 3" stroke="#2a2a3a" />
                  <XAxis dataKey="time" stroke="#64748b" fontSize={12} />
                  <YAxis stroke="#64748b" fontSize={12} />
                  <Tooltip
                    contentStyle={{
                      backgroundColor: '#111118',
                      border: '1px solid #2a2a3a',
                      borderRadius: '8px',
                    }}
                  />
                  <Area
                    type="monotone"
                    dataKey="success"
                    stroke="#10b981"
                    fill="url(#successGradient)"
                    strokeWidth={2}
                  />
                  <Area
                    type="monotone"
                    dataKey="failed"
                    stroke="#ef4444"
                    fill="#ef4444"
                    fillOpacity={0.1}
                    strokeWidth={2}
                  />
                </AreaChart>
              </ResponsiveContainer>
            </div>
          </Card>

          {/* Volume by Route */}
          <Card>
            <div className="flex items-center justify-between mb-6">
              <div>
                <h3 className="font-semibold">Volume by Route</h3>
                <p className="text-sm text-[var(--color-text-muted)]">Tasks processed per route</p>
              </div>
              <Badge>
                <Calendar size={12} className="mr-1" />
                Last 7 days
              </Badge>
            </div>
            <div className="h-64">
              <ResponsiveContainer width="100%" height="100%">
                <BarChart data={volumeData}>
                  <CartesianGrid strokeDasharray="3 3" stroke="#2a2a3a" />
                  <XAxis dataKey="name" stroke="#64748b" fontSize={12} />
                  <YAxis stroke="#64748b" fontSize={12} />
                  <Tooltip
                    contentStyle={{
                      backgroundColor: '#111118',
                      border: '1px solid #2a2a3a',
                      borderRadius: '8px',
                    }}
                  />
                  <Bar dataKey="arb_eth" name="ARB → ETH" fill="#6366f1" radius={[4, 4, 0, 0]} />
                  <Bar dataKey="arb_gnosis" name="ARB → Gnosis" fill="#8b5cf6" radius={[4, 4, 0, 0]} />
                </BarChart>
              </ResponsiveContainer>
            </div>
          </Card>

          {/* Latency Chart */}
          <Card>
            <div className="flex items-center justify-between mb-6">
              <div>
                <h3 className="font-semibold">Response Latency</h3>
                <p className="text-sm text-[var(--color-text-muted)]">Average task execution time</p>
              </div>
              <div className="flex items-center gap-2 text-sm">
                <Clock size={14} className="text-[var(--color-text-muted)]" />
                <span>Avg: 1.3s</span>
              </div>
            </div>
            <div className="h-64">
              <ResponsiveContainer width="100%" height="100%">
                <LineChart data={performanceData}>
                  <CartesianGrid strokeDasharray="3 3" stroke="#2a2a3a" />
                  <XAxis dataKey="time" stroke="#64748b" fontSize={12} />
                  <YAxis stroke="#64748b" fontSize={12} unit="s" />
                  <Tooltip
                    contentStyle={{
                      backgroundColor: '#111118',
                      border: '1px solid #2a2a3a',
                      borderRadius: '8px',
                    }}
                    formatter={(value) => [`${value}s`, 'Latency']}
                  />
                  <Line
                    type="monotone"
                    dataKey="latency"
                    stroke="#f59e0b"
                    strokeWidth={2}
                    dot={{ fill: '#f59e0b', strokeWidth: 0 }}
                  />
                </LineChart>
              </ResponsiveContainer>
            </div>
          </Card>

          {/* Task Distribution */}
          <Card>
            <div className="flex items-center justify-between mb-6">
              <div>
                <h3 className="font-semibold">Task Distribution</h3>
                <p className="text-sm text-[var(--color-text-muted)]">Breakdown by task type</p>
              </div>
              <BarChart3 size={18} className="text-[var(--color-text-muted)]" />
            </div>
            <div className="flex items-center gap-8">
              <div className="h-48 w-48">
                <ResponsiveContainer width="100%" height="100%">
                  <PieChart>
                    <Pie
                      data={taskDistribution}
                      cx="50%"
                      cy="50%"
                      innerRadius={40}
                      outerRadius={70}
                      paddingAngle={2}
                      dataKey="value"
                    >
                      {taskDistribution.map((entry, index) => (
                        <Cell key={index} fill={entry.color} />
                      ))}
                    </Pie>
                    <Tooltip
                      contentStyle={{
                        backgroundColor: '#111118',
                        border: '1px solid #2a2a3a',
                        borderRadius: '8px',
                      }}
                    />
                  </PieChart>
                </ResponsiveContainer>
              </div>
              <div className="flex-1 space-y-2">
                {taskDistribution.map((task) => (
                  <div key={task.name} className="flex items-center justify-between text-sm">
                    <div className="flex items-center gap-2">
                      <div className="w-3 h-3 rounded-full" style={{ backgroundColor: task.color }} />
                      <span>{task.name}</span>
                    </div>
                    <span className="text-[var(--color-text-muted)]">{task.value}%</span>
                  </div>
                ))}
              </div>
            </div>
          </Card>
        </div>

        {/* Error Log */}
        <Card padding="none">
          <div className="p-4 border-b border-[var(--color-border)] flex items-center justify-between">
            <div className="flex items-center gap-2">
              <XCircle size={18} className="text-red-400" />
              <h3 className="font-semibold">Recent Errors</h3>
            </div>
            <Badge variant="error">{errorLog.length} errors</Badge>
          </div>
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="border-b border-[var(--color-border)] text-left text-sm text-[var(--color-text-muted)]">
                  <th className="px-4 py-3 font-medium">Timestamp</th>
                  <th className="px-4 py-3 font-medium">Error Type</th>
                  <th className="px-4 py-3 font-medium">Route</th>
                  <th className="px-4 py-3 font-medium">Epoch</th>
                  <th className="px-4 py-3 font-medium">Status</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-[var(--color-border)]">
                {errorLog.map((error, i) => (
                  <tr key={i} className="hover:bg-[var(--color-surface-2)] transition-colors">
                    <td className="px-4 py-3 text-sm font-mono">{error.timestamp}</td>
                    <td className="px-4 py-3">
                      <Badge variant="error" size="sm">{error.type}</Badge>
                    </td>
                    <td className="px-4 py-3 text-sm">{error.route}</td>
                    <td className="px-4 py-3 text-sm">{error.epoch}</td>
                    <td className="px-4 py-3">
                      <div className="flex items-center gap-1 text-sm text-emerald-400">
                        <CheckCircle2 size={14} />
                        Resolved
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </Card>
      </div>
    </Layout>
  )
}
