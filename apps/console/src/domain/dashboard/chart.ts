export interface Bar {
	x: number
	y: number
	width: number
	height: number
	series: number
}

export const CHART_WIDTH = 280
export const CHART_HEIGHT = 88
export const CHART_DAYS = 14
const BAR_WIDTH = 14
const DAY_MS = 86_400_000

const gapFor = (count: number): number =>
	count > 1 ? (CHART_WIDTH - BAR_WIDTH * count) / (count - 1) : 0

export function stackedBars(
	rows: ReadonlyArray<ReadonlyArray<number>>,
): ReadonlyArray<Bar> {
	const gap = gapFor(rows.length)
	const max = Math.max(0, ...rows.map((row) => row.reduce((a, b) => a + b, 0)))
	if (max <= 0) return []
	return rows.flatMap((row, index) => {
		let y = CHART_HEIGHT
		const x = index * (BAR_WIDTH + gap)
		return row.flatMap((value, series) => {
			const h = (value / max) * (CHART_HEIGHT - 4)
			if (h <= 0) return []
			y -= h
			return [
				{ x, y, width: BAR_WIDTH, height: Math.max(h - 0.6, 0.6), series },
			]
		})
	})
}

export interface DayWindow {
	from: string
	to: string
	days: ReadonlyArray<string>
}

export function dayWindow(now: Date, count = CHART_DAYS): DayWindow {
	const end = Date.UTC(
		now.getUTCFullYear(),
		now.getUTCMonth(),
		now.getUTCDate() + 1,
	)
	const days = Array.from({ length: count }, (_, index) =>
		new Date(end - (count - index) * DAY_MS).toISOString().slice(0, 10),
	)
	return {
		from: new Date(end - count * DAY_MS).toISOString(),
		to: new Date(end).toISOString(),
		days,
	}
}

export function dailySeries<Row extends { key: string }>(
	rows: ReadonlyArray<Row>,
	days: ReadonlyArray<string>,
	width: number,
	pick: (row: Row) => ReadonlyArray<number>,
): ReadonlyArray<ReadonlyArray<number>> {
	const byDay = new Map(rows.map((row) => [row.key.slice(0, 10), row]))
	return days.map((day) => {
		const row = byDay.get(day)
		return row ? pick(row) : Array.from({ length: width }, () => 0)
	})
}

const AXIS_FORMAT = new Intl.DateTimeFormat('fr-FR', {
	day: 'numeric',
	month: 'short',
	timeZone: 'UTC',
})

export function axisDates(days: ReadonlyArray<string>): ReadonlyArray<string> {
	const picks = [0, Math.floor((days.length - 1) / 2), days.length - 1]
	return [...new Set(picks)].flatMap((index) => {
		const day = days[index]
		return day ? [AXIS_FORMAT.format(new Date(`${day}T00:00:00Z`))] : []
	})
}
