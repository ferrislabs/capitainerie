import { useState } from 'react'
import { formatCost } from '#/domain/dashboard/format'
import { donutArcs, type Slice, slotColor } from '#/domain/metrics/charts'
import { cn } from '#/lib/utils'

interface Props {
	title: string
	slices: ReadonlyArray<Slice>
	total: number
	/** Emplacement de couleur d'une part, par clé. */
	slotOf: (key: string) => number
}

const RADIUS = 38
const CIRCUMFERENCE = 2 * Math.PI * RADIUS
const PERCENT = new Intl.NumberFormat('fr-FR', {
	style: 'percent',
	maximumFractionDigits: 0,
})

const colorOf = (slice: Slice, slotOf: (key: string) => number) =>
	slice.other ? 'var(--chart-other)' : slotColor(slotOf(slice.key))

export function ShareDonut({ title, slices, total, slotOf }: Props) {
	const [active, setActive] = useState<string | null>(null)
	const arcs = donutArcs(slices, CIRCUMFERENCE)
	const focus = slices.find((slice) => slice.key === active)
	return (
		<section className="min-w-0 rounded-lg border bg-card px-3 py-2.5">
			<h3 className="m-0 mb-2 text-xs font-semibold">{title}</h3>
			<div className="flex flex-wrap items-center gap-4">
				<div className="relative size-[116px] shrink-0">
					<svg
						viewBox="0 0 100 100"
						role="img"
						aria-label={`${title}, ${slices.length} parts`}
						className="size-full -rotate-90"
					>
						{arcs.map((arc, index) => {
							const slice = slices[index]
							if (!slice) return null
							return (
								<circle
									key={arc.key}
									cx="50"
									cy="50"
									r={RADIUS}
									fill="none"
									strokeWidth="12"
									strokeDasharray={`${arc.length} ${CIRCUMFERENCE - arc.length}`}
									strokeDashoffset={-arc.offset}
									style={{ stroke: colorOf(slice, slotOf) }}
									className={cn(
										'transition-opacity',
										active !== null && active !== arc.key && 'opacity-35',
									)}
								/>
							)
						})}
					</svg>
					<div className="pointer-events-none absolute inset-0 grid place-items-center text-center">
						<span className="figures text-[13px] leading-tight font-semibold">
							{formatCost(focus ? focus.value : total)}
							<span className="block text-[10.5px] font-normal text-muted-foreground">
								{focus ? PERCENT.format(focus.share) : 'total'}
							</span>
						</span>
					</div>
				</div>
				<ul className="m-0 min-w-0 flex-1 list-none p-0 text-[12px]">
					{slices.map((slice) => (
						<li
							key={slice.key}
							onMouseEnter={() => setActive(slice.key)}
							onMouseLeave={() => setActive(null)}
							className="flex items-center gap-2 py-[3px]"
						>
							<i
								aria-hidden="true"
								className="size-2 shrink-0 rounded-full"
								style={{ backgroundColor: colorOf(slice, slotOf) }}
							/>
							<span className="min-w-0 flex-1 truncate">{slice.label}</span>
							<span className="figures font-mono text-[11.5px] text-muted-foreground">
								{PERCENT.format(slice.share)}
							</span>
							<span className="figures w-16 text-right font-mono text-[11.5px]">
								{formatCost(slice.value)}
							</span>
						</li>
					))}
				</ul>
			</div>
		</section>
	)
}
