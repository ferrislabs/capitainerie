import type { Schemas } from '#/api/api.client'
import { formatLatency } from '#/domain/runs/format'
import { stepKindLabel } from '#/domain/runs/step-labels'
import {
	STEP_STATUS_LABELS,
	STEP_STATUS_TEXT,
	StepStatusIcon,
} from '#/domain/runs/ui/step-visual'
import { cn } from '#/lib/utils'

interface Props {
	steps: ReadonlyArray<Schemas.StepResponse>
	selectedSeq: number | undefined
	onSelect: (seq: number) => void
	onKeyDown: (key: string) => boolean
}

export function stepOptionId(seq: number): string {
	return `run-step-${seq}`
}

export function StepList({ steps, selectedSeq, onSelect, onKeyDown }: Props) {
	return (
		<ul
			aria-label="Étapes"
			onKeyDown={(event) => {
				if (!onKeyDown(event.key)) return
				event.preventDefault()
			}}
		>
			{steps.map((step) => {
				const selected = step.seq === selectedSeq
				return (
					<li key={step.seq} className="border-b last:border-b-0">
						<button
							type="button"
							id={stepOptionId(step.seq)}
							aria-current={selected ? 'true' : undefined}
							tabIndex={selected || selectedSeq === undefined ? 0 : -1}
							onClick={() => onSelect(step.seq)}
							className={cn(
								'flex w-full items-center gap-3 px-4 py-3 text-left focus-visible:outline-2 focus-visible:outline-offset-[-2px] focus-visible:outline-ring',
								selected ? 'bg-muted' : 'hover:bg-muted/60',
							)}
						>
							<StepStatusIcon
								status={step.status}
								className={STEP_STATUS_TEXT[step.status]}
							/>
							<span className="flex min-w-0 flex-1 flex-col">
								<span className="font-medium">{stepKindLabel(step.kind)}</span>
								<span className={cn('text-xs', STEP_STATUS_TEXT[step.status])}>
									{STEP_STATUS_LABELS[step.status]}
								</span>
							</span>
							<span className="text-[13px] font-medium figures">
								{step.latency_ms == null ? '' : formatLatency(step.latency_ms)}
							</span>
						</button>
					</li>
				)
			})}
		</ul>
	)
}
