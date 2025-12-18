import clsx from 'clsx'

interface BadgeProps {
  children: React.ReactNode
  variant?: 'default' | 'success' | 'warning' | 'error' | 'info'
  size?: 'sm' | 'md'
}

export function Badge({ children, variant = 'default', size = 'md' }: BadgeProps) {
  return (
    <span className={clsx(
      'inline-flex items-center font-medium rounded-full',
      size === 'sm' && 'px-2 py-0.5 text-xs',
      size === 'md' && 'px-2.5 py-1 text-xs',
      variant === 'default' && 'bg-[var(--color-surface-2)] text-[var(--color-text-muted)]',
      variant === 'success' && 'bg-emerald-500/20 text-emerald-400',
      variant === 'warning' && 'bg-amber-500/20 text-amber-400',
      variant === 'error' && 'bg-red-500/20 text-red-400',
      variant === 'info' && 'bg-blue-500/20 text-blue-400'
    )}>
      {children}
    </span>
  )
}
