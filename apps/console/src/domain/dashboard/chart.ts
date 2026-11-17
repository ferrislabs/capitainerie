export interface Bar {
	x: number
	y: number
	width: number
	height: number
	series: number
}

export const CHART_WIDTH = 280
export const CHART_HEIGHT = 88
const BAR_WIDTH = 14

const gapFor = (count: number): number =>
	(CHART_WIDTH - BAR_WIDTH * count) / (count - 1)

export function stackedBars(
	rows: ReadonlyArray<ReadonlyArray<number>>,
): ReadonlyArray<Bar> {
	const gap = gapFor(rows.length)
	const max = Math.max(...rows.map((row) => row.reduce((a, b) => a + b, 0)))
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

export function rateBars(values: ReadonlyArray<number>): ReadonlyArray<Bar> {
	const gap = gapFor(values.length)
	return values.map((value, index) => {
		const h = ((value - 80) / 20) * (CHART_HEIGHT - 4)
		return {
			x: index * (BAR_WIDTH + gap),
			y: CHART_HEIGHT - h,
			width: BAR_WIDTH,
			height: h,
			series: 0,
		}
	})
}
