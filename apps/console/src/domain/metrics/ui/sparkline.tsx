interface Props {
	values: ReadonlyArray<number>
	label: string
}

const WIDTH = 120
const HEIGHT = 28

/** Courbe de tendance d'une tuile : ligne de 2 px et aire discrète, sans axe. */
export function Sparkline({ values, label }: Props) {
	const max = Math.max(...values, 0)
	if (values.length < 2 || max <= 0) return null
	const step = WIDTH / (values.length - 1)
	const points = values.map((value, index) => [
		index * step,
		HEIGHT - 2 - (value / max) * (HEIGHT - 4),
	])
	const line = points.map(([x, y]) => `${x?.toFixed(1)},${y?.toFixed(1)}`)
	return (
		<svg
			viewBox={`0 0 ${WIDTH} ${HEIGHT}`}
			preserveAspectRatio="none"
			role="img"
			aria-label={label}
			className="mt-1 block h-7 w-full"
		>
			<polygon
				points={`0,${HEIGHT} ${line.join(' ')} ${WIDTH},${HEIGHT}`}
				className="fill-primary/10"
			/>
			<polyline
				points={line.join(' ')}
				fill="none"
				strokeWidth="2"
				strokeLinejoin="round"
				strokeLinecap="round"
				vectorEffect="non-scaling-stroke"
				className="stroke-primary"
			/>
		</svg>
	)
}
