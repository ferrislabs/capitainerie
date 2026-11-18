import { useState } from 'react'
import { DataTable, Td, Th, Tr } from '#/domain/audit/ui/data-table'
import { formatCost, formatTokens } from '#/domain/dashboard/format'
import { cumulate, type DayPoint, niceMax } from '#/domain/metrics/charts'
import { cn } from '#/lib/utils'

interface Props {
	points: ReadonlyArray<DayPoint>
	/** Période précédente, jour pour jour ; absente tant qu'elle charge. */
	previous?: ReadonlyArray<DayPoint>
}

type Mode = 'jour' | 'cumul'

const MODES: ReadonlyArray<{ value: Mode; label: string }> = [
	{ value: 'jour', label: 'Par jour' },
	{ value: 'cumul', label: 'Cumulé' },
]

const path = (values: ReadonlyArray<number>, max: number): string =>
	values
		.map((value, index) => {
			const x = ((index + 0.5) / values.length) * 100
			const y = 100 - (value / max) * 100
			return `${x.toFixed(2)},${y.toFixed(2)}`
		})
		.join(' ')

const MICROS = 1_000_000
const GRID = [1, 0.5, 0]

const SHORT = new Intl.DateTimeFormat('fr-FR', {
	day: 'numeric',
	month: 'short',
	timeZone: 'UTC',
})
const LONG = new Intl.DateTimeFormat('fr-FR', {
	weekday: 'long',
	day: 'numeric',
	month: 'long',
	timeZone: 'UTC',
})

const asDate = (day: string) => new Date(`${day}T00:00:00Z`)

function axisIndexes(count: number): number[] {
	return [...new Set([0, Math.floor((count - 1) / 2), count - 1])]
}

export function DailyChart({ points: daily, previous: previousDaily }: Props) {
	const [active, setActive] = useState<number | null>(null)
	const [table, setTable] = useState(false)
	const [mode, setMode] = useState<Mode>('jour')
	const cumul = mode === 'cumul'
	const points = cumul ? cumulate(daily) : daily
	const previous = previousDaily
		? (cumul ? cumulate(previousDaily) : previousDaily).slice(0, points.length)
		: undefined
	const max = niceMax(
		Math.max(
			...points.map((p) => p.cost_micros),
			...(previous ?? []).map((p) => p.cost_micros),
		) / MICROS,
	)
	const current = active === null ? undefined : points[active]
	const before = active === null ? undefined : previous?.[active]
	return (
		<section className="min-w-0 rounded-lg border bg-card px-3 py-2.5">
			<div className="mb-2 flex items-center gap-2">
				<h3 className="m-0 text-xs font-semibold">Évolution du coût</h3>
				<div className="flex gap-0.5 rounded-md bg-secondary p-0.5 text-[11.5px]">
					{MODES.map((item) => (
						<button
							key={item.value}
							type="button"
							aria-pressed={mode === item.value}
							onClick={() => setMode(item.value)}
							className={cn(
								'rounded-[4px] px-2 py-0.5 focus-visible:outline-2 focus-visible:outline-ring',
								mode === item.value
									? 'bg-card font-medium shadow-xs'
									: 'text-muted-foreground',
							)}
						>
							{item.label}
						</button>
					))}
				</div>
				<button
					type="button"
					aria-pressed={table}
					onClick={() => setTable((value) => !value)}
					className="ml-auto text-[11.5px] text-link hover:underline focus-visible:outline-2 focus-visible:outline-ring"
				>
					{table ? 'Voir le graphique' : 'Voir le tableau'}
				</button>
			</div>
			{table ? (
				<div className="max-h-[188px] overflow-y-auto">
					<DataTable>
						<thead>
							<tr>
								<Th>Jour</Th>
								<Th>Entrée</Th>
								<Th>Sortie</Th>
								<Th>Coût</Th>
							</tr>
						</thead>
						<tbody>
							{points.map((point) => (
								<Tr key={point.day}>
									<Td className="figures font-mono text-[11.5px]">
										{point.day}
									</Td>
									<Td className="figures font-mono text-[11.5px]">
										{formatTokens(point.input_tokens)}
									</Td>
									<Td className="figures font-mono text-[11.5px]">
										{formatTokens(point.output_tokens)}
									</Td>
									<Td className="figures font-mono text-[11.5px]">
										{formatCost(point.cost_micros)}
									</Td>
								</Tr>
							))}
						</tbody>
					</DataTable>
				</div>
			) : (
				<div className="flex gap-2">
					<div
						aria-hidden="true"
						className="flex h-[160px] w-11 shrink-0 flex-col justify-between text-right font-mono text-[10.5px] text-faint"
					>
						{GRID.map((part) => (
							<span key={part} className="leading-none">
								{formatCost(part * max * MICROS)}
							</span>
						))}
					</div>
					<div className="min-w-0 flex-1">
						<div
							role="img"
							aria-label={`Évolution du coût, ${points.length} jours`}
							className="relative h-[160px]"
						>
							<div
								aria-hidden="true"
								className="pointer-events-none absolute inset-0 flex flex-col justify-between"
							>
								{GRID.map((part) => (
									<div
										key={part}
										className={cn(
											'border-t',
											part === 0 ? 'border-border' : 'border-border/50',
										)}
									/>
								))}
							</div>
							{cumul ? (
								<svg
									viewBox="0 0 100 100"
									preserveAspectRatio="none"
									aria-hidden="true"
									className="pointer-events-none absolute inset-0 size-full"
								>
									<polygon
										points={`0,100 ${path(
											points.map((p) => p.cost_micros / MICROS),
											max,
										)} 100,100`}
										className="fill-primary/12"
									/>
									<polyline
										points={path(
											points.map((p) => p.cost_micros / MICROS),
											max,
										)}
										fill="none"
										strokeWidth="2"
										strokeLinejoin="round"
										vectorEffect="non-scaling-stroke"
										className="stroke-primary"
									/>
								</svg>
							) : null}
							{previous ? (
								<svg
									viewBox="0 0 100 100"
									preserveAspectRatio="none"
									aria-hidden="true"
									className="pointer-events-none absolute inset-0 z-[1] size-full"
								>
									<polyline
										points={path(
											previous.map((p) => p.cost_micros / MICROS),
											max,
										)}
										fill="none"
										strokeWidth="1.5"
										strokeDasharray="4 3"
										vectorEffect="non-scaling-stroke"
										className="stroke-muted-foreground"
									/>
								</svg>
							) : null}
							<div className="relative flex h-full items-end gap-[2px]">
								{points.map((point, index) => (
									<button
										key={point.day}
										type="button"
										aria-label={`${LONG.format(asDate(point.day))} : ${formatCost(point.cost_micros)}`}
										onMouseEnter={() => setActive(index)}
										onMouseLeave={() => setActive(null)}
										onFocus={() => setActive(index)}
										onBlur={() => setActive(null)}
										className="group flex h-full min-w-0 flex-1 items-end focus-visible:outline-2 focus-visible:outline-ring"
									>
										<span
											className={cn(
												'block w-full rounded-t-[4px] transition-opacity',
												cumul ? 'bg-transparent' : 'bg-primary',
												active !== null && active !== index && 'opacity-45',
											)}
											style={{
												height: `${(point.cost_micros / MICROS / max) * 100}%`,
												minHeight: point.cost_micros > 0 ? 2 : 0,
											}}
										/>
									</button>
								))}
							</div>
							{current && active !== null ? (
								<output
									className="pointer-events-none absolute -top-1 z-10 block w-max -translate-y-full rounded-md border bg-popover px-2 py-1.5 text-[11.5px] shadow-sm"
									style={{
										left: `${((active + 0.5) / points.length) * 100}%`,
										transform: `translate(${active > points.length / 2 ? '-100%' : '0'}, -100%)`,
									}}
								>
									<b className="block font-medium">
										{LONG.format(asDate(current.day))}
									</b>
									<span className="figures font-mono">
										{formatCost(current.cost_micros)}
									</span>
									{before ? (
										<span className="figures block font-mono text-muted-foreground">
											{formatCost(before.cost_micros)} avant
										</span>
									) : null}
									<span className="block text-muted-foreground">
										{formatTokens(current.input_tokens)} entrée ·{' '}
										{formatTokens(current.output_tokens)} sortie
									</span>
								</output>
							) : null}
						</div>
						{previous ? (
							<div className="mt-1 flex items-center gap-1.5 text-[11px] text-muted-foreground">
								<i
									aria-hidden="true"
									className="inline-block w-4 border-t-[1.5px] border-dashed border-muted-foreground"
								/>
								Période précédente
							</div>
						) : null}
						<div className="mt-1 flex justify-between font-mono text-[10.5px] text-faint">
							{axisIndexes(points.length).map((index) => (
								<span key={index}>
									{SHORT.format(asDate(points[index]?.day ?? ''))}
								</span>
							))}
						</div>
					</div>
				</div>
			)}
		</section>
	)
}
