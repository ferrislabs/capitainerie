const numberFormat = new Intl.NumberFormat('fr-FR')
const costFormat = new Intl.NumberFormat('fr-FR', {
	minimumFractionDigits: 4,
	maximumFractionDigits: 4,
})
const secondsFormat = new Intl.NumberFormat('fr-FR', {
	minimumFractionDigits: 1,
	maximumFractionDigits: 1,
})

export function formatTokens(count: number): string {
	return numberFormat.format(count)
}

export function formatCost(micros: number): string {
	return `${costFormat.format(micros / 1_000_000)} €`
}

export function formatLatency(ms: number): string {
	if (ms < 1000) return `${numberFormat.format(Math.round(ms))} ms`
	return `${secondsFormat.format(ms / 1000)} s`
}

export function runDurationMs(run: {
	started_at?: string | null | undefined
	finished_at?: string | null | undefined
}): number | null {
	if (!run.started_at || !run.finished_at) return null
	const ms = Date.parse(run.finished_at) - Date.parse(run.started_at)
	return Number.isNaN(ms) || ms < 0 ? null : ms
}

export function formatElapsed(ms: number): string {
	const totalSeconds = Math.max(0, Math.floor(ms / 1000))
	if (totalSeconds < 60) return `${totalSeconds} s`
	const seconds = String(totalSeconds % 60).padStart(2, '0')
	if (totalSeconds < 3600)
		return `${Math.floor(totalSeconds / 60)} min ${seconds} s`
	const minutes = String(Math.floor((totalSeconds % 3600) / 60)).padStart(
		2,
		'0',
	)
	return `${Math.floor(totalSeconds / 3600)} h ${minutes} min`
}
