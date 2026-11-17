import {
	addTrigger,
	notify,
	patchSchedule,
	regenerateSecret,
	removeTrigger,
	setTriggerEnabled,
} from '#/domain/routines/feature/routines-store'
import { nextRuns } from '#/domain/routines/schedule'
import type { Routine, WebhookTrigger } from '#/domain/routines/types'
import { TriggersTab } from '#/domain/routines/ui/triggers-tab'

interface Props {
	routine: Routine
	now: Date
}

function copyUrl(trigger: WebhookTrigger) {
	navigator.clipboard?.writeText(trigger.url).catch(() => undefined)
	notify('URL copiée')
}

export function TriggersPanel({ routine, now }: Props) {
	return (
		<TriggersTab
			triggers={routine.triggers}
			nextRunsOf={(schedule) => nextRuns(schedule, now, 3)}
			onPatchSchedule={(triggerId, patch) =>
				patchSchedule(routine.id, triggerId, patch)
			}
			onToggle={(triggerId, enabled) =>
				setTriggerEnabled(routine.id, triggerId, enabled)
			}
			onRemove={(triggerId) => removeTrigger(routine.id, triggerId)}
			onCopy={copyUrl}
			onRegenerate={(triggerId) => regenerateSecret(routine.id, triggerId)}
			onAdd={(kind) => addTrigger(routine.id, kind)}
		/>
	)
}
