import { type Bar, CHART_HEIGHT, CHART_WIDTH } from '#/domain/dashboard/chart'
import { cn } from '#/lib/utils'

export interface LegendEntry {
	label: string
	fill: string
	dot: string
}

export type PanelStatus = 'loading' | 'error' | 'ready'

interface Props {
	title: string
	status: PanelStatus
	dates: ReadonlyArray<string>
	bars: ReadonlyArray<Bar>
	legend: ReadonlyArray<LegendEntry>
}

export function ChartPanel({ title, status, dates, bars, legend }: Props) {
	return (
		<section className="min-w-0 rounded-lg border bg-card px-3 py-2.5">
			<h3 className="m-0 text-xs font-medium text-muted-foreground">{title}</h3>
			<div className="mb-1.5 text-[11px] text-faint">14 derniers jours</div>
			{status === 'ready' && bars.length > 0 ? (
				<svg
					viewBox={`0 0 ${CHART_WIDTH} ${CHART_HEIGHT}`}
					preserveAspectRatio="none"
					role="img"
					aria-label={`${title}, 14 derniers jours`}
					className="block h-[88px] w-full"
				>
					{bars.map((bar) => (
						<rect
							key={`${bar.x}-${bar.series}`}
							x={bar.x.toFixed(1)}
							y={bar.y.toFixed(1)}
							width={bar.width}
							height={bar.height.toFixed(1)}
							rx="1.2"
							className={legend[bar.series]?.fill}
						/>
					))}
				</svg>
			) : (
				<output className="grid h-[88px] place-items-center text-[11px] text-faint">
					{status === 'loading'
						? 'Chargement…'
						: status === 'error'
							? 'Données indisponibles'
							: 'Aucune activité sur la période'}
				</output>
			)}
			<div className="mt-[3px] flex justify-between font-mono text-[10.5px] text-faint">
				{dates.map((date) => (
					<span key={date}>{date}</span>
				))}
			</div>
			<ul className="mt-[7px] flex flex-wrap gap-x-2.5 gap-y-[3px] text-[11px] text-muted-foreground">
				{legend.map((entry) => (
					<li key={entry.label}>
						<i
							aria-hidden="true"
							className={cn(
								'mr-1 inline-block size-[7px] rounded-full',
								entry.dot,
							)}
						/>
						{entry.label}
					</li>
				))}
			</ul>
		</section>
	)
}
