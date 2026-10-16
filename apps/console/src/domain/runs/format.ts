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
