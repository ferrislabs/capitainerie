import { useNavigate } from '@tanstack/react-router'
import { useState } from 'react'
import { Page } from '#/components/layout/page'
import { useRoutineFormOptions } from '#/domain/routines/feature/use-routine-context'
import { useCreateRoutine } from '#/domain/routines/hooks/use-routines'
import {
	createBody,
	describeRoutineError,
	emptyDraft,
	type RoutineDraft,
} from '#/domain/routines/model'
import { RoutineForm } from '#/domain/routines/ui/routine-form'

const INITIAL = emptyDraft()

export function RoutineNewPage() {
	const navigate = useNavigate()
	const { agents, projects } = useRoutineFormOptions()
	const create = useCreateRoutine()
	const [error, setError] = useState<string | null>(null)

	const submit = (draft: RoutineDraft) => {
		setError(null)
		create.mutate(
			{ body: createBody(draft) },
			{
				onSuccess: (routine) =>
					void navigate({
						to: '/routines/$routineId/{-$tab}',
						params: { routineId: routine.id },
					}),
				onError: (failure) => setError(describeRoutineError(failure)),
			},
		)
	}

	return (
		<Page>
			<header className="mb-4">
				<h1 className="m-0 text-xl leading-[1.2] font-semibold tracking-[-0.01em]">
					Nouvelle routine
				</h1>
				<span className="text-muted-foreground">
					Travail récurrent qui devient une tâche auditable à chaque passage
				</span>
			</header>
			<RoutineForm
				mode="create"
				initial={INITIAL}
				agents={agents}
				projects={projects}
				pending={create.isPending}
				error={error}
				onSubmit={submit}
				onCancel={() => void navigate({ to: '/routines' })}
			/>
		</Page>
	)
}
