import { useMemo } from 'react'
import { Button } from '#/components/ui/button'
import {
	discardEditDraft,
	saveEditDraft,
	setEditDraft,
	useRoutinesState,
} from '#/domain/routines/feature/routines-store'
import { useFormOptions } from '#/domain/routines/feature/use-routine-context'
import { settingsOf } from '#/domain/routines/routine-draft'
import type { Routine } from '#/domain/routines/types'
import { RoutineForm } from '#/domain/routines/ui/routine-form'

interface Props {
	routine: Routine
}

export function SettingsTab({ routine }: Props) {
	const stored = useRoutinesState((s) => s.editDrafts[routine.id])
	const base = useMemo(() => settingsOf(routine), [routine])
	const draft = stored ?? base
	const options = useFormOptions(draft.agent)
	return (
		<div>
			<RoutineForm
				settings={draft}
				{...options}
				onChange={(patch) => setEditDraft(routine.id, { ...draft, ...patch })}
			/>
			<div className="flex justify-end gap-2">
				<Button
					type="button"
					variant="outline"
					onClick={() => discardEditDraft(routine.id)}
				>
					Annuler
				</Button>
				<Button type="button" onClick={() => saveEditDraft(routine.id)}>
					Enregistrer
				</Button>
			</div>
		</div>
	)
}
