import type { DailyRuns } from '#/domain/audit/mock/audit-data'
import { stackedBars } from '#/domain/audit/stacked-bars'

interface Props {
	days: ReadonlyArray<DailyRuns>
}

const WIDTH = 280
const HEIGHT = 88
const FILLS = ['fill-signal-success', 'fill-signal-danger'] as const

export function RunsChart({ days }: Props) {
	const segments = stackedBars(
		days.map((day) => [day.succeeded, day.failed]),
		{ width: WIDTH, height: HEIGHT, barWidth: 14 },
	)
	const succeeded = days.reduce((sum, day) => sum + day.succeeded, 0)
	const failed = days.reduce((sum, day) => sum + day.failed, 0)
	const first = days[0]?.day
	const last = days.at(-1)?.day
	const summary = `Exécutions par jour sur ${days.length} jours, du ${first} au ${last} : ${succeeded} réussies, ${failed} en échec.`
	return (
		<figure className="flex flex-col gap-2">
			<svg
				viewBox={`0 0 ${WIDTH} ${HEIGHT}`}
				preserveAspectRatio="none"
				role="img"
				aria-label={summary}
				className="h-24 w-full"
			>
				<title>{summary}</title>
				{segments.map((segment) => (
					<rect
						key={`${segment.x}-${segment.series}`}
						x={segment.x}
						y={segment.y}
						width={segment.width}
						height={segment.height}
						rx={1.2}
						className={FILLS[segment.series]}
					/>
				))}
			</svg>
			<figcaption className="flex justify-between text-xs text-muted-foreground">
				<span>{first}</span>
				<span>
					<span className="mr-3 inline-flex items-center gap-1">
						<i className="size-2 rounded-sm bg-signal-success" />
						Réussies
					</span>
					<span className="inline-flex items-center gap-1">
						<i className="size-2 rounded-sm bg-signal-danger" />
						En échec
					</span>
				</span>
				<span>{last}</span>
			</figcaption>
		</figure>
	)
}
