import { Link, useNavigate } from '@tanstack/react-router'
import { DemoBanner } from '#/components/layout/demo-banner'
import { Notice } from '#/components/layout/notice'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
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
			<PageHeader
				title="Nouvelle routine"
				description="Travail récurrent qui devient une tâche auditable à chaque passage"
			/>
			<Notice message={notice} />
			<RoutineForm
				settings={draft.settings}
				{...options}
				onChange={(patch) =>
					setNewDraft({ ...draft, settings: { ...draft.settings, ...patch } })
				}
				between={
					<SectionCard title="Déclencheur : planning">
						<div className="flex flex-col gap-3">
							<ScheduleFields
								schedule={draft.schedule}
								onChange={(patch) =>
									setNewDraft({
										...draft,
										schedule: { ...draft.schedule, ...patch },
									})
								}
							/>
							<div className="max-w-md">
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
							</div>
							<div className="flex flex-col gap-1.5">
								<h3 className="text-xs text-muted-foreground">
									Prochains passages
								</h3>
								<NextRuns
									runs={nextRuns(draft.schedule, now, 3)}
									empty={CRON_NOTE}
								/>
							</div>
						</div>
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
