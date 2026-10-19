import type { Schemas } from '#/api/api.client'
import { formatLatency } from '#/domain/runs/format'
import { stepKindLabel } from '#/domain/runs/step-labels'
import {
	type Axis,
	blockPosition,
	buildTicks,
} from '#/domain/runs/timeline-axis'
import {
	STEP_BLOCK,
	STEP_STATUS_LABELS,
	StepStatusIcon,
} from '#/domain/runs/ui/step-visual'
import { cn } from '#/lib/utils'

const MIN_BLOCK_WIDTH = '1.75rem'

interface Props {
	steps: ReadonlyArray<Schemas.StepResponse>
	axis: Axis
	nowMs: number
	selectedSeq: number | undefined
	onSelect: (seq: number) => void
}

function blockLabel(step: Schemas.StepResponse, nowMs: number): string {
	const end = step.finished_at ? Date.parse(step.finished_at) : nowMs
	const ms = step.latency_ms ?? Math.max(0, end - Date.parse(step.started_at))
	return `${stepKindLabel(step.kind)}, ${STEP_STATUS_LABELS[step.status].toLowerCase()}, ${formatLatency(ms)}`
}

export function RunFrise({ steps, axis, nowMs, selectedSeq, onSelect }: Props) {
	const ticks = buildTicks(axis.totalMs)

	return (
		<div className="px-6 pt-2 pb-3">
			<div className="relative h-6" aria-hidden="true">
				{ticks.map((tick) => (
					<span
						key={tick.ms}
						className="absolute top-0 -translate-x-1/2 text-[11px] text-muted-foreground figures whitespace-nowrap"
						style={{ left: `${tick.leftPct}%` }}
					>
						{tick.label}
					</span>
				))}
			</div>
			<div className="relative">
				<div
					aria-hidden="true"
					className="pointer-events-none absolute inset-0"
				>
					{ticks.map((tick) => (
						<span
							key={tick.ms}
							className="absolute inset-y-0 w-px bg-border"
							style={{ left: `${tick.leftPct}%` }}
						/>
					))}
				</div>
				<ul
					aria-label="Frise des étapes"
					className="relative flex flex-col gap-1.5"
				>
					{steps.map((step) => {
						const { leftPct, widthPct } = blockPosition(step, axis, nowMs)
						const selected = step.seq === selectedSeq
						return (
							<li key={step.seq} className="relative h-8">
								<button
									type="button"
									aria-label={blockLabel(step, nowMs)}
									aria-pressed={selected}
									onClick={() => onSelect(step.seq)}
									data-seq={step.seq}
									className={cn(
										'absolute top-0 flex h-8 items-center gap-1.5 overflow-hidden rounded-md border px-2 text-xs font-medium whitespace-nowrap transition-shadow duration-150',
										'focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-ring',
										STEP_BLOCK[step.status],
										selected &&
											'ring-2 ring-ring ring-offset-1 ring-offset-card',
									)}
									style={{
										left: `${leftPct}%`,
										width: `${widthPct}%`,
										minWidth: MIN_BLOCK_WIDTH,
									}}
								>
									<StepStatusIcon status={step.status} className="size-3.5" />
									<span className="truncate">{stepKindLabel(step.kind)}</span>
								</button>
							</li>
						)
					})}
				</ul>
			</div>
		</div>
	)
}
