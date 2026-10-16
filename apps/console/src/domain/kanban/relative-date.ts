const formatter = new Intl.RelativeTimeFormat('fr-FR', { numeric: 'auto' })

const UNITS: ReadonlyArray<readonly [Intl.RelativeTimeFormatUnit, number]> = [
	['year', 31_536_000],
	['month', 2_592_000],
	['day', 86_400],
	['hour', 3_600],
	['minute', 60],
]

export function relativeDate(iso: string, now: number = Date.now()): string {
	const seconds = Math.round((new Date(iso).getTime() - now) / 1000)
	const abs = Math.abs(seconds)
	for (const [unit, size] of UNITS) {
		if (abs >= size) return formatter.format(Math.round(seconds / size), unit)
	}
	return formatter.format(0, 'second')
}
