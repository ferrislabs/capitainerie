import { Badge } from '#/components/ui/badge'
import { cn } from '#/lib/utils'

export type StatusTone =
	| 'neutral'
	| 'running'
	| 'success'
	| 'danger'
	| 'attention'

const DOTS: Record<StatusTone, string> = {
	neutral: 'bg-signal-neutral',
	running: 'bg-signal-running',
	success: 'bg-signal-success',
	danger: 'bg-signal-danger',
	attention: 'bg-signal-attention-ink',
}

interface Props {
	tone: StatusTone
	label: string
	className?: string
}

export function StatusBadge({ tone, label, className }: Props) {
	return (
		<Badge variant={tone} className={className}>
			<span
				aria-hidden="true"
				className={cn(
					'size-1.5 rounded-full',
					DOTS[tone],
					tone === 'running' && 'motion-safe:animate-signal',
				)}
			/>
			{label}
		</Badge>
	)
}
