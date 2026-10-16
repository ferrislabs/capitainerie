import { useRealtimeState } from '#/hooks/use-realtime'
import type { RealtimeState } from '#/lib/realtime'

const INDICATORS: Record<RealtimeState, { label: string; dot: string }> = {
	connected: { label: 'Temps réel', dot: 'bg-emerald-500' },
	degraded: { label: 'Mode dégradé', dot: 'bg-amber-500' },
	offline: { label: 'Hors ligne', dot: 'bg-muted-foreground/50' },
}

export function RealtimeIndicator() {
	const { label, dot } = INDICATORS[useRealtimeState()]

	return (
		<output className="flex items-center gap-2 text-xs text-muted-foreground">
			<span className={`size-2 rounded-full ${dot}`} aria-hidden="true" />
			{label}
		</output>
	)
}
