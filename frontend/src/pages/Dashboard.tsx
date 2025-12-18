import { useState } from 'react'
import { Layout } from '../components/Layout'
import { Card, StatCard } from '../components/Card'
import { Button } from '../components/Button'
import { Badge } from '../components/Badge'
import {
  Activity,
  Clock,
  CheckCircle2,
  XCircle,
  AlertTriangle,
  RefreshCw,
  Power,
  Settings,
  ChevronDown,
  Play,
  Zap,
  Shield,
  Send,
  Download,
  Eye
} from 'lucide-react'
import clsx from 'clsx'

interface LogEntry {
  id: string
  timestamp: string
  level: 'success' | 'info' | 'warning' | 'error'
  message: string
  route: string
}

interface Task {
  id: string
  kind: string
  epoch: number
  executeAfter: string
  status: 'pending' | 'running' | 'completed' | 'failed'
}

const mockLogs: LogEntry[] = [
  { id: '1', timestamp: '12:45:23', level: 'success', message: 'SaveSnapshot executed for epoch 1247', route: 'ARB_TO_ETH' },
  { id: '2', timestamp: '12:44:18', level: 'info', message: 'ValidateClaim: Epoch 1246 VALID', route: 'ARB_TO_ETH' },
  { id: '3', timestamp: '12:43:05', level: 'success', message: 'WithdrawDeposit completed for epoch 1245', route: 'ARB_TO_GNOSIS' },
  { id: '4', timestamp: '12:42:30', level: 'warning', message: 'RootNotConfirmed for epoch 1244, rescheduling +1h', route: 'ARB_TO_ETH' },
  { id: '5', timestamp: '12:41:15', level: 'info', message: 'Indexer sync complete', route: 'ARB_TO_GNOSIS' },
  { id: '6', timestamp: '12:40:02', level: 'success', message: 'VerifySnapshot succeeded for epoch 1243', route: 'ARB_TO_ETH' },
  { id: '7', timestamp: '12:38:45', level: 'error', message: 'Insufficient funds for challenge, will retry in 15min', route: 'ARB_TO_GNOSIS' },
  { id: '8', timestamp: '12:37:22', level: 'success', message: 'StartVerification executed for epoch 1242', route: 'ARB_TO_ETH' },
]

const mockTasks: Task[] = [
  { id: '1', kind: 'VerifySnapshot', epoch: 1247, executeAfter: 'in 5 min', status: 'pending' },
  { id: '2', kind: 'ExecuteRelay', epoch: 1246, executeAfter: 'in 1h 23m', status: 'pending' },
  { id: '3', kind: 'WithdrawDeposit', epoch: 1245, executeAfter: 'ready', status: 'running' },
]

const taskActions = [
  { id: 'save-snapshot', label: 'Save Snapshot', icon: Download, description: 'Save current epoch snapshot to inbox' },
  { id: 'claim', label: 'Claim', icon: Zap, description: 'Claim epoch on outbox (requires MAKE_CLAIMS=true)', requiresClaims: true },
  { id: 'validate-claim', label: 'Validate Claim', icon: Eye, description: 'Compare claimed state root with inbox' },
  { id: 'challenge', label: 'Challenge', icon: Shield, description: 'Challenge fraudulent claim with deposit' },
  { id: 'send-snapshot', label: 'Send Snapshot', icon: Send, description: 'Send snapshot via native bridge' },
  { id: 'start-verification', label: 'Start Verification', icon: Play, description: 'Begin verification period' },
  { id: 'verify-snapshot', label: 'Verify Snapshot', icon: CheckCircle2, description: 'Finalize snapshot verification' },
  { id: 'execute-relay', label: 'Execute Relay', icon: Activity, description: 'Execute L2→L1 message relay' },
  { id: 'withdraw-deposit', label: 'Withdraw Deposit', icon: Download, description: 'Withdraw deposit to honest party' },
]

export function Dashboard() {
  const [isRunning, setIsRunning] = useState(true)
  const [selectedRoute, setSelectedRoute] = useState('all')
  const [makeClaims] = useState(false)
  const [expandedAction, setExpandedAction] = useState<string | null>(null)
  const [actionLoading, setActionLoading] = useState<string | null>(null)
  const [showConfirmModal, setShowConfirmModal] = useState<string | null>(null)

  const handleAction = async (actionId: string) => {
    setShowConfirmModal(null)
    setActionLoading(actionId)
    await new Promise(r => setTimeout(r, 2000))
    setActionLoading(null)
    setExpandedAction(null)
  }

  const filteredLogs = selectedRoute === 'all' 
    ? mockLogs 
    : mockLogs.filter(l => l.route === selectedRoute)

  return (
    <Layout>
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {/* Header */}
        <div className="flex flex-col lg:flex-row lg:items-center lg:justify-between gap-4 mb-8">
          <div>
            <h1 className="text-2xl font-bold">Dashboard</h1>
            <p className="text-[var(--color-text-muted)] mt-1">Monitor and control your VEA Validator</p>
          </div>
          <div className="flex items-center gap-3">
            <Button 
              variant="secondary" 
              icon={<RefreshCw size={16} />}
              onClick={() => window.location.reload()}
            >
              Refresh
            </Button>
            <Button
              variant={isRunning ? 'danger' : 'primary'}
              icon={<Power size={16} />}
              onClick={() => setIsRunning(!isRunning)}
            >
              {isRunning ? 'Stop' : 'Start'}
            </Button>
            <Button variant="ghost" icon={<Settings size={16} />} />
          </div>
        </div>

        {/* Status Banner */}
        <Card className="mb-8">
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
            <div className="flex items-center gap-4">
              <div className={clsx(
                'w-12 h-12 rounded-xl flex items-center justify-center',
                isRunning ? 'bg-emerald-500/20' : 'bg-red-500/20'
              )}>
                {isRunning ? (
                  <Activity size={24} className="text-emerald-400" />
                ) : (
                  <XCircle size={24} className="text-red-400" />
                )}
              </div>
              <div>
                <div className="flex items-center gap-2">
                  <h2 className="text-lg font-semibold">Validator Status</h2>
                  <Badge variant={isRunning ? 'success' : 'error'}>
                    {isRunning ? 'Running' : 'Stopped'}
                  </Badge>
                </div>
                <p className="text-sm text-[var(--color-text-muted)] mt-0.5">
                  Wallet: 0x1234...5678 • 2 routes active
                </p>
              </div>
            </div>
            <div className="flex items-center gap-6 text-sm">
              <div>
                <p className="text-[var(--color-text-muted)]">MAKE_CLAIMS</p>
                <p className={makeClaims ? 'text-emerald-400' : 'text-[var(--color-text-muted)]'}>
                  {makeClaims ? 'Enabled' : 'Disabled'}
                </p>
              </div>
              <div>
                <p className="text-[var(--color-text-muted)]">Sync Status</p>
                <p className="text-emerald-400">On Sync</p>
              </div>
            </div>
          </div>
        </Card>

        {/* KPIs */}
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-8">
          <StatCard
            label="Uptime"
            value="14d 6h"
            change="+2.3% from last month"
            changeType="positive"
            icon={<Clock size={20} />}
          />
          <StatCard
            label="Success Rate"
            value="99.8%"
            change="1,247 / 1,250 tasks"
            changeType="positive"
            icon={<CheckCircle2 size={20} />}
          />
          <StatCard
            label="Pending Tasks"
            value={mockTasks.length}
            change="Next in 5 min"
            changeType="neutral"
            icon={<Activity size={20} />}
          />
          <StatCard
            label="Challenges"
            value="2"
            change="0 missed"
            changeType="positive"
            icon={<Shield size={20} />}
          />
        </div>

        <div className="grid lg:grid-cols-3 gap-8">
          {/* Main Content */}
          <div className="lg:col-span-2 space-y-8">
            {/* Logs */}
            <Card padding="none">
              <div className="p-4 border-b border-[var(--color-border)] flex items-center justify-between">
                <h3 className="font-semibold">Activity Logs</h3>
                <select
                  value={selectedRoute}
                  onChange={(e) => setSelectedRoute(e.target.value)}
                  className="bg-[var(--color-surface-2)] border border-[var(--color-border)] rounded-lg px-3 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-[var(--color-primary)]"
                >
                  <option value="all">All Routes</option>
                  <option value="ARB_TO_ETH">Arbitrum → Ethereum</option>
                  <option value="ARB_TO_GNOSIS">Arbitrum → Gnosis</option>
                </select>
              </div>
              <div className="divide-y divide-[var(--color-border)] max-h-[400px] overflow-y-auto">
                {filteredLogs.map((log) => (
                  <div key={log.id} className="px-4 py-3 hover:bg-[var(--color-surface-2)] transition-colors">
                    <div className="flex items-start gap-3">
                      <span className="text-xs text-[var(--color-text-muted)] font-mono mt-0.5">
                        {log.timestamp}
                      </span>
                      <Badge 
                        variant={
                          log.level === 'success' ? 'success' :
                          log.level === 'error' ? 'error' :
                          log.level === 'warning' ? 'warning' : 'info'
                        }
                        size="sm"
                      >
                        {log.level.toUpperCase()}
                      </Badge>
                      <div className="flex-1 min-w-0">
                        <p className="text-sm truncate">{log.message}</p>
                        <p className="text-xs text-[var(--color-text-muted)] mt-0.5">{log.route}</p>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </Card>

            {/* Pending Tasks */}
            <Card padding="none">
              <div className="p-4 border-b border-[var(--color-border)]">
                <h3 className="font-semibold">Pending Tasks</h3>
              </div>
              <div className="divide-y divide-[var(--color-border)]">
                {mockTasks.map((task) => (
                  <div key={task.id} className="px-4 py-3 flex items-center justify-between">
                    <div className="flex items-center gap-3">
                      <div className={clsx(
                        'w-2 h-2 rounded-full',
                        task.status === 'running' && 'bg-amber-400 animate-pulse',
                        task.status === 'pending' && 'bg-[var(--color-text-muted)]'
                      )} />
                      <div>
                        <p className="text-sm font-medium">{task.kind}</p>
                        <p className="text-xs text-[var(--color-text-muted)]">Epoch {task.epoch}</p>
                      </div>
                    </div>
                    <div className="text-right">
                      <p className="text-sm">{task.executeAfter}</p>
                      <Badge variant={task.status === 'running' ? 'warning' : 'default'} size="sm">
                        {task.status}
                      </Badge>
                    </div>
                  </div>
                ))}
                {mockTasks.length === 0 && (
                  <div className="px-4 py-8 text-center text-[var(--color-text-muted)]">
                    No pending tasks
                  </div>
                )}
              </div>
            </Card>
          </div>

          {/* Sidebar - Actions */}
          <div className="space-y-4">
            <Card padding="none">
              <div className="p-4 border-b border-[var(--color-border)]">
                <h3 className="font-semibold">Manual Actions</h3>
                <p className="text-xs text-[var(--color-text-muted)] mt-1">
                  Execute tasks manually (use with caution)
                </p>
              </div>
              <div className="divide-y divide-[var(--color-border)]">
                {taskActions.map((action) => {
                  const isDisabled = action.requiresClaims && !makeClaims
                  const isExpanded = expandedAction === action.id
                  const isLoading = actionLoading === action.id

                  if (isDisabled) return null

                  return (
                    <div key={action.id}>
                      <button
                        onClick={() => setExpandedAction(isExpanded ? null : action.id)}
                        className="w-full px-4 py-3 flex items-center justify-between hover:bg-[var(--color-surface-2)] transition-colors text-left"
                        disabled={isLoading}
                      >
                        <div className="flex items-center gap-3">
                          <action.icon size={16} className="text-[var(--color-primary)]" />
                          <span className="text-sm font-medium">{action.label}</span>
                        </div>
                        <ChevronDown 
                          size={16} 
                          className={clsx(
                            'text-[var(--color-text-muted)] transition-transform',
                            isExpanded && 'rotate-180'
                          )} 
                        />
                      </button>
                      {isExpanded && (
                        <div className="px-4 pb-4 space-y-3">
                          <p className="text-xs text-[var(--color-text-muted)]">
                            {action.description}
                          </p>
                          <div className="space-y-2">
                            <label className="block">
                              <span className="text-xs text-[var(--color-text-muted)]">Route</span>
                              <select className="mt-1 w-full bg-[var(--color-surface-2)] border border-[var(--color-border)] rounded-lg px-3 py-2 text-sm">
                                <option>ARB_TO_ETH</option>
                                <option>ARB_TO_GNOSIS</option>
                              </select>
                            </label>
                            <label className="block">
                              <span className="text-xs text-[var(--color-text-muted)]">Epoch</span>
                              <input 
                                type="number" 
                                placeholder="e.g. 1247"
                                className="mt-1 w-full bg-[var(--color-surface-2)] border border-[var(--color-border)] rounded-lg px-3 py-2 text-sm"
                              />
                            </label>
                          </div>
                          <Button
                            size="sm"
                            className="w-full"
                            loading={isLoading}
                            onClick={() => setShowConfirmModal(action.id)}
                          >
                            Execute {action.label}
                          </Button>
                        </div>
                      )}
                    </div>
                  )
                })}
              </div>
            </Card>

            {/* Alerts */}
            <Card>
              <div className="flex items-center gap-2 mb-4">
                <AlertTriangle size={16} className="text-amber-400" />
                <h3 className="font-semibold">Alerts</h3>
              </div>
              <div className="space-y-3">
                <div className="p-3 bg-amber-500/10 border border-amber-500/20 rounded-lg">
                  <p className="text-sm text-amber-400 font-medium">Low WETH Balance</p>
                  <p className="text-xs text-[var(--color-text-muted)] mt-1">
                    Gnosis WETH below recommended threshold
                  </p>
                </div>
                <div className="p-3 bg-blue-500/10 border border-blue-500/20 rounded-lg">
                  <p className="text-sm text-blue-400 font-medium">Pending Relay</p>
                  <p className="text-xs text-[var(--color-text-muted)] mt-1">
                    ExecuteRelay waiting for root confirmation
                  </p>
                </div>
              </div>
            </Card>
          </div>
        </div>
      </div>

      {/* Confirmation Modal */}
      {showConfirmModal && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
          <Card className="max-w-md w-full">
            <div className="flex items-center gap-3 mb-4">
              <div className="p-2 bg-amber-500/20 rounded-lg">
                <AlertTriangle size={20} className="text-amber-400" />
              </div>
              <h3 className="text-lg font-semibold">Confirm Action</h3>
            </div>
            <p className="text-[var(--color-text-muted)] mb-6">
              Are you sure you want to execute this action? This will submit a transaction to the blockchain.
            </p>
            <div className="flex gap-3">
              <Button 
                variant="secondary" 
                className="flex-1"
                onClick={() => setShowConfirmModal(null)}
              >
                Cancel
              </Button>
              <Button 
                className="flex-1"
                onClick={() => handleAction(showConfirmModal)}
              >
                Confirm
              </Button>
            </div>
          </Card>
        </div>
      )}
    </Layout>
  )
}
