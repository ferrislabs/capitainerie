import { type RoutineRun, runOutcome } from '#/domain/routines/model'
import { Sev } from '#/domain/sources/ui/mockup'

export function RunBadge({ run }: { run: RoutineRun }) {
	const outcome = runOutcome(run)
	return <Sev tone={outcome.tone}>{outcome.label}</Sev>
}
