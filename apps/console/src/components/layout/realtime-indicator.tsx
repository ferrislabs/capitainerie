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

export function RealtimeIndicator() {
	const { label, dot, pulse } = INDICATORS[useRealtimeState()]

	return (
		<output
			title={label}
			className="flex h-7 items-center gap-2 rounded-md px-1.5 text-xs font-medium text-muted-foreground sm:px-2"
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
			<span className="max-sm:sr-only">{label}</span>
		</output>
	)
}
