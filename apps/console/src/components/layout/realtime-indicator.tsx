import { useRealtimeState } from '#/hooks/use-realtime'
import type { RealtimeState } from '#/lib/realtime'
import { cn } from '#/lib/utils'

const INDICATORS: Record<
	RealtimeState,
	{ label: string; dot: string; pulse: boolean }
> = {
	connected: { label: 'Temps réel', dot: 'bg-signal-success', pulse: true },
	degraded: { label: 'Mode dégradé', dot: 'bg-signal-attention', pulse: false },
	offline: { label: 'Hors ligne', dot: 'bg-muted-foreground/60', pulse: false },
}

interface Props {
	compact?: boolean
}

export function RealtimeIndicator({ compact = false }: Props) {
	const { label, dot, pulse } = INDICATORS[useRealtimeState()]

	return (
		<output
			title={label}
			className={cn(
				'flex h-7 items-center gap-2 rounded-md px-2 text-xs text-rail-label',
				compact && 'justify-center px-0',
			)}
		>
			<span className="relative flex size-2" aria-hidden="true">
				{pulse ? (
					<span
						className={cn(
							'absolute inset-0 rounded-full opacity-60 motion-safe:animate-ping',
							dot,
						)}
					/>
				) : null}
				<span className={cn('relative size-2 rounded-full', dot)} />
			</span>
			<span className={cn(compact && 'sr-only')}>{label}</span>
		</output>
	)
}
