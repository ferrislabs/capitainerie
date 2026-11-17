export interface BarSegment {
	series: number
	x: number
	y: number
	width: number
	height: number
}

export interface StackedBarsOptions {
	width: number
	height: number
	barWidth: number
}

/** Segments empilés par colonne, du bas vers le haut ; les valeurs nulles sont ignorées. */
export function stackedBars(
	rows: ReadonlyArray<ReadonlyArray<number>>,
	{ width, height, barWidth }: StackedBarsOptions,
): BarSegment[] {
	if (rows.length === 0) return []
	const gap =
		rows.length > 1 ? (width - barWidth * rows.length) / (rows.length - 1) : 0
	const max = Math.max(...rows.map((row) => row.reduce((a, b) => a + b, 0)))
	if (max <= 0) return []
	const segments: BarSegment[] = []
	rows.forEach((row, column) => {
		let y = height
		row.forEach((value, series) => {
			if (value <= 0) return
			const segmentHeight = (value / max) * (height - 4)
			y -= segmentHeight
			segments.push({
				series,
				x: column * (barWidth + gap),
				y,
				width: barWidth,
				height: Math.max(segmentHeight - 0.6, 0.6),
			})
		})
	})
	return segments
}
