import { Link } from 'react-router-dom'
import { Layout } from '../components/Layout'
import { Card } from '../components/Card'
import { Button } from '../components/Button'
import { 
  Shield, 
  Zap, 
  CheckCircle2, 
  ArrowRight,
  Clock,
  Activity,
  Lock,
  Globe,
  Users,
  TrendingUp
} from 'lucide-react'

const features = [
  {
    icon: Shield,
    title: 'Fraud Detection',
    description: 'Automatically monitors claims and challenges fraudulent state roots to protect the bridge.'
  },
  {
    icon: Zap,
    title: 'Automated Execution',
    description: 'Handles snapshots, verifications, and message relays without manual intervention.'
  },
  {
    icon: Clock,
    title: '24/7 Monitoring',
    description: 'Continuous epoch watching and event indexing ensures nothing is missed.'
  },
  {
    icon: Lock,
    title: 'Deposit Management',
    description: 'Automatic withdrawal of deposits to the honest party after verification.'
  },
  {
    icon: Globe,
    title: 'Multi-Route Support',
    description: 'Supports Arbitrum → Ethereum and Arbitrum → Gnosis bridges simultaneously.'
  },
  {
    icon: Activity,
    title: 'Real-time Logs',
    description: 'Detailed logging of all operations with clear success/error feedback.'
  }
]

const stats = [
  { value: '99.9%', label: 'Uptime' },
  { value: '< 2s', label: 'Avg Response' },
  { value: '1000+', label: 'Epochs Validated' },
  { value: '0', label: 'Missed Challenges' }
]

const testimonials = [
  {
    quote: "VEA Validator has been running flawlessly for months. The automatic challenge detection gives us peace of mind.",
    author: "Bridge Operator",
    role: "DeFi Protocol"
  },
  {
    quote: "The dashboard makes it easy to monitor multiple routes at once. Couldn't run our bridge without it.",
    author: "Infrastructure Lead",
    role: "L2 Network"
  }
]

export function Landing() {
  return (
    <Layout variant="landing">
      {/* Hero Section */}
      <section className="relative overflow-hidden">
        <div className="absolute inset-0 bg-gradient-to-br from-indigo-900/20 via-transparent to-purple-900/20" />
        <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[800px] h-[800px] bg-indigo-500/10 rounded-full blur-3xl" />
        
        <div className="relative max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-24 lg:py-32">
          <div className="text-center max-w-4xl mx-auto">
            <div className="inline-flex items-center gap-2 px-4 py-2 bg-[var(--color-surface)] border border-[var(--color-border)] rounded-full text-sm text-[var(--color-text-muted)] mb-8">
              <span className="w-2 h-2 bg-emerald-400 rounded-full animate-pulse" />
              Actively securing VEA bridges
            </div>
            
            <h1 className="text-4xl sm:text-5xl lg:text-6xl font-bold leading-tight">
              Secure Cross-Chain
              <span className="block bg-gradient-to-r from-indigo-400 to-purple-400 bg-clip-text text-transparent">
                Message Validation
              </span>
            </h1>
            
            <p className="mt-6 text-lg sm:text-xl text-[var(--color-text-muted)] max-w-2xl mx-auto">
              Rust-powered validator for the VEA protocol. Monitor claims, challenge fraud, 
              and relay L2→L1 messages automatically.
            </p>
            
            <div className="mt-10 flex flex-col sm:flex-row items-center justify-center gap-4">
              <Link to="/dashboard">
                <Button size="lg" icon={<ArrowRight size={18} />}>
                  Launch Dashboard
                </Button>
              </Link>
              <a href="https://github.com" target="_blank" rel="noopener noreferrer">
                <Button variant="secondary" size="lg">
                  View on GitHub
                </Button>
              </a>
            </div>
          </div>
        </div>
      </section>

      {/* Stats Section */}
      <section className="border-y border-[var(--color-border)] bg-[var(--color-surface)]">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12">
          <div className="grid grid-cols-2 lg:grid-cols-4 gap-8">
            {stats.map((stat, i) => (
              <div key={i} className="text-center">
                <p className="text-3xl sm:text-4xl font-bold bg-gradient-to-r from-indigo-400 to-purple-400 bg-clip-text text-transparent">
                  {stat.value}
                </p>
                <p className="mt-2 text-sm text-[var(--color-text-muted)]">{stat.label}</p>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Features Section */}
      <section className="py-24">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="text-center mb-16">
            <h2 className="text-3xl sm:text-4xl font-bold">Everything You Need</h2>
            <p className="mt-4 text-lg text-[var(--color-text-muted)] max-w-2xl mx-auto">
              A complete solution for validating and securing VEA bridge operations.
            </p>
          </div>
          
          <div className="grid md:grid-cols-2 lg:grid-cols-3 gap-6">
            {features.map((feature, i) => (
              <Card key={i} className="hover:border-[var(--color-primary)] transition-colors">
                <div className="p-3 w-fit bg-[var(--color-primary)]/10 rounded-lg mb-4">
                  <feature.icon size={24} className="text-[var(--color-primary)]" />
                </div>
                <h3 className="text-lg font-semibold mb-2">{feature.title}</h3>
                <p className="text-[var(--color-text-muted)] text-sm">{feature.description}</p>
              </Card>
            ))}
          </div>
        </div>
      </section>

      {/* Dashboard Preview */}
      <section className="py-24 bg-[var(--color-surface)] border-y border-[var(--color-border)]">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="text-center mb-16">
            <h2 className="text-3xl sm:text-4xl font-bold">Powerful Dashboard</h2>
            <p className="mt-4 text-lg text-[var(--color-text-muted)] max-w-2xl mx-auto">
              Monitor your validator, view logs, and control operations from a single interface.
            </p>
          </div>
          
          {/* Mock Dashboard Preview */}
          <div className="relative rounded-2xl border border-[var(--color-border)] bg-[var(--color-bg)] overflow-hidden shadow-2xl">
            <div className="border-b border-[var(--color-border)] px-4 py-3 flex items-center gap-2">
              <div className="w-3 h-3 rounded-full bg-red-500" />
              <div className="w-3 h-3 rounded-full bg-yellow-500" />
              <div className="w-3 h-3 rounded-full bg-green-500" />
              <span className="ml-4 text-sm text-[var(--color-text-muted)]">VEA Validator Dashboard</span>
            </div>
            <div className="p-6 space-y-6">
              {/* Mock KPIs */}
              <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
                {[
                  { label: 'Status', value: 'Running', color: 'text-emerald-400' },
                  { label: 'Uptime', value: '14d 6h 23m', color: 'text-white' },
                  { label: 'Pending Tasks', value: '3', color: 'text-amber-400' },
                  { label: 'Last Sync', value: '2s ago', color: 'text-white' }
                ].map((item, i) => (
                  <div key={i} className="bg-[var(--color-surface)] border border-[var(--color-border)] rounded-lg p-4">
                    <p className="text-xs text-[var(--color-text-muted)]">{item.label}</p>
                    <p className={`text-lg font-semibold mt-1 ${item.color}`}>{item.value}</p>
                  </div>
                ))}
              </div>
              
              {/* Mock Logs */}
              <div className="bg-[var(--color-surface)] border border-[var(--color-border)] rounded-lg p-4">
                <p className="text-xs text-[var(--color-text-muted)] mb-3">Recent Activity</p>
                <div className="space-y-2 font-mono text-xs">
                  <div className="flex gap-3">
                    <span className="text-[var(--color-text-muted)]">12:45:23</span>
                    <span className="text-emerald-400">[SUCCESS]</span>
                    <span>SaveSnapshot executed for epoch 1247</span>
                  </div>
                  <div className="flex gap-3">
                    <span className="text-[var(--color-text-muted)]">12:44:18</span>
                    <span className="text-blue-400">[INFO]</span>
                    <span>ValidateClaim: Epoch 1246 VALID</span>
                  </div>
                  <div className="flex gap-3">
                    <span className="text-[var(--color-text-muted)]">12:43:05</span>
                    <span className="text-emerald-400">[SUCCESS]</span>
                    <span>WithdrawDeposit completed for epoch 1245</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* Testimonials */}
      <section className="py-24">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="text-center mb-16">
            <h2 className="text-3xl sm:text-4xl font-bold">Trusted by Teams</h2>
            <p className="mt-4 text-lg text-[var(--color-text-muted)]">
              See what bridge operators are saying about VEA Validator.
            </p>
          </div>
          
          <div className="grid md:grid-cols-2 gap-8">
            {testimonials.map((t, i) => (
              <Card key={i} padding="lg">
                <div className="flex gap-1 mb-4">
                  {[1,2,3,4,5].map(n => (
                    <CheckCircle2 key={n} size={16} className="text-[var(--color-primary)]" />
                  ))}
                </div>
                <p className="text-lg mb-6">"{t.quote}"</p>
                <div className="flex items-center gap-3">
                  <div className="w-10 h-10 rounded-full bg-gradient-to-br from-indigo-500 to-purple-600 flex items-center justify-center">
                    <Users size={18} className="text-white" />
                  </div>
                  <div>
                    <p className="font-medium">{t.author}</p>
                    <p className="text-sm text-[var(--color-text-muted)]">{t.role}</p>
                  </div>
                </div>
              </Card>
            ))}
          </div>
        </div>
      </section>

      {/* CTA Section */}
      <section className="py-24 bg-gradient-to-br from-indigo-900/50 to-purple-900/50">
        <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 text-center">
          <TrendingUp size={48} className="mx-auto text-[var(--color-primary)] mb-6" />
          <h2 className="text-3xl sm:text-4xl font-bold mb-6">Ready to Secure Your Bridge?</h2>
          <p className="text-lg text-[var(--color-text-muted)] mb-10">
            Get started with VEA Validator today and ensure your cross-chain messages are always protected.
          </p>
          <div className="flex flex-col sm:flex-row items-center justify-center gap-4">
            <Link to="/dashboard">
              <Button size="lg" icon={<ArrowRight size={18} />}>
                Launch Dashboard
              </Button>
            </Link>
            <a href="#" target="_blank" rel="noopener noreferrer">
              <Button variant="secondary" size="lg">
                Read Documentation
              </Button>
            </a>
          </div>
        </div>
      </section>
    </Layout>
  )
}
