import clsx from 'clsx'

interface CardProps {
  children: React.ReactNode
  className?: string
  padding?: 'none' | 'sm' | 'md' | 'lg'
}

export function Card({ children, className, padding = 'md' }: CardProps) {
  return (
    <div className={clsx(
      'bg-[var(--color-surface)] border border-[var(--color-border)] rounded-xl',
      padding === 'sm' && 'p-4',
      padding === 'md' && 'p-6',
      padding === 'lg' && 'p-8',
      className
    )}>
      {children}
    </div>
  )
}

interface StatCardProps {
  label: string
  value: string | number
  change?: string
  changeType?: 'positive' | 'negative' | 'neutral'
  icon?: React.ReactNode
}

export function StatCard({ label, value, change, changeType = 'neutral', icon }: StatCardProps) {
  return (
    <Card>
      <div className="flex items-start justify-between">
        <div>
          <p className="text-sm text-[var(--color-text-muted)]">{label}</p>
          <p className="text-2xl font-bold mt-1">{value}</p>
          {change && (
            <p className={clsx(
              'text-sm mt-2',
              changeType === 'positive' && 'text-[var(--color-success)]',
              changeType === 'negative' && 'text-[var(--color-error)]',
              changeType === 'neutral' && 'text-[var(--color-text-muted)]'
            )}>
              {change}
            </p>
          )}
        </div>
        {icon && (
          <div className="p-3 bg-[var(--color-surface-2)] rounded-lg text-[var(--color-primary)]">
            {icon}
          </div>
        )}
      </div>
    </Card>
  )
}
