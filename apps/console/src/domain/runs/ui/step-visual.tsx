import { Check, CircleDashed, Loader, X } from 'lucide-react'
import type { Schemas } from '#/api/api.client'
import { cn } from '#/lib/utils'

export type StepStatus = Schemas.StepStatusResponse

export const STEP_STATUS_LABELS: Record<StepStatus, string> = {
	running: 'En cours',
	succeeded: 'Terminé',
	failed: 'Échec',
	skipped: 'Ignoré',
}

export const STEP_STATUS_TEXT: Record<StepStatus, string> = {
	running: 'text-signal-running',
	succeeded: 'text-signal-success',
	failed: 'text-signal-danger',
	skipped: 'text-muted-foreground',
}

export const STEP_BLOCK: Record<StepStatus, string> = {
	running: 'border-signal-running bg-signal-running-soft text-signal-running',
	succeeded: 'border-signal-success bg-signal-success-soft text-signal-success',
	failed: 'border-signal-danger bg-signal-danger-soft text-signal-danger',
	skipped: 'border-dashed border-input bg-muted text-muted-foreground',
}

export function StepStatusIcon({
	status,
	className,
}: {
	status: StepStatus
	className?: string
}) {
	const classes = cn('size-4 shrink-0', className)
	switch (status) {
		case 'running':
			return (
				<Loader
					aria-hidden
					className={cn(classes, 'motion-safe:animate-spin')}
				/>
			)
		case 'succeeded':
			return <Check aria-hidden className={classes} />
		case 'failed':
			return <X aria-hidden className={classes} />
		case 'skipped':
			return <CircleDashed aria-hidden className={classes} />
	}
}
