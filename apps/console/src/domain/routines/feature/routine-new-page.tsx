import { Link, useNavigate } from '@tanstack/react-router'
import { DemoBanner } from '#/components/layout/demo-banner'
import { Notice } from '#/components/layout/notice'
import { Page } from '#/components/layout/page'
import { SectionCard } from '#/components/layout/section-card'
import { Button } from '#/components/ui/button'
import {
	blankDraft,
	createRoutine,
	setNewDraft,
	useRoutinesState,
} from '#/domain/routines/feature/routines-store'
import {
	useFormOptions,
	useNow,
	useRoutineNotice,
} from '#/domain/routines/feature/use-routine-context'
import { nextRuns } from '#/domain/routines/schedule'
import { TIMEZONES } from '#/domain/routines/types'
import { CRON_NOTE, NextRuns } from '#/domain/routines/ui/next-runs'
import { RoutineForm } from '#/domain/routines/ui/routine-form'
import { ScheduleFields } from '#/domain/routines/ui/schedule-fields'
import { SelectField } from '#/domain/routines/ui/select-field'

const BLANK = blankDraft()

export function RoutineNewPage() {
	const stored = useRoutinesState((s) => s.newDraft)
	const draft = stored ?? BLANK
	const notice = useRoutineNotice()
	const now = useNow()
	const navigate = useNavigate()
	const options = useFormOptions(draft.settings.agent)

	const create = () => {
		const id = createRoutine()
		if (id)
			navigate({
				to: '/routines/$routineId/{-$tab}',
				params: { routineId: id },
			})
	}

	return (
		<Page>
			<DemoBanner />
			<header className="mb-3">
				<h1 className="m-0 text-xl leading-[1.2] font-semibold tracking-[-0.01em]">
					Nouvelle routine
				</h1>
				<span className="text-muted-foreground">
					Travail récurrent qui devient une tâche auditable à chaque passage
				</span>
			</header>
			<Notice message={notice} />
			<RoutineForm
				settings={draft.settings}
				{...options}
				onChange={(patch) =>
					setNewDraft({ ...draft, settings: { ...draft.settings, ...patch } })
				}
				between={
					<SectionCard title="Déclencheur : planning" flush>
						<ScheduleFields
							schedule={draft.schedule}
							onChange={(patch) =>
								setNewDraft({
									...draft,
									schedule: { ...draft.schedule, ...patch },
								})
							}
						/>
						<SelectField
							label="Fuseau"
							value={draft.settings.timezone}
							options={TIMEZONES}
							onChange={(timezone) =>
								setNewDraft({
									...draft,
									settings: { ...draft.settings, timezone },
								})
							}
						/>
						<div className="px-3 pb-1 text-muted-foreground">
							Prochains passages
						</div>
						<NextRuns
							runs={nextRuns(draft.schedule, now, 3)}
							empty={CRON_NOTE}
						/>
					</SectionCard>
				}
			/>
			<div className="flex justify-end gap-2">
				<Button asChild variant="outline">
					<Link to="/routines">Annuler</Link>
				</Button>
				<Button type="button" onClick={create}>
					Créer la routine
				</Button>
			</div>
		</Page>
	)
}
