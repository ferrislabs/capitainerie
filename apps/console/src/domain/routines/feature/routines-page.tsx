import { Link } from '@tanstack/react-router'
import { Plus } from 'lucide-react'
import { DemoBanner } from '#/components/layout/demo-banner'
import { Notice } from '#/components/layout/notice'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { Button } from '#/components/ui/button'
import {
	launchRoutine,
	toggleRoutine,
	useRoutinesState,
} from '#/domain/routines/feature/routines-store'
import { useRoutineNotice } from '#/domain/routines/feature/use-routine-context'
import { scheduleLabel } from '#/domain/routines/routine-view'
import { RoutinesTable } from '#/domain/routines/ui/routines-table'

export function RoutinesPage() {
	const routines = useRoutinesState((s) => s.routines)
	const notice = useRoutineNotice()
	const rows = routines.map((routine) => ({
		routine,
		scheduleLabel: routine.listSchedule ?? scheduleLabel(routine),
	}))
	return (
		<Page>
			<DemoBanner />
			<PageHeader
				title="Routines"
				description={`Travail récurrent qui devient des tâches auditables · ${routines.length} routines`}
				actions={
					<>
						<Button asChild variant="outline">
							<Link to="/audit/{-$tab}" params={{ tab: 'runs' }}>
								Toutes les exécutions
							</Link>
						</Button>
						<Button asChild>
							<Link to="/routines/nouvelle">
								<Plus aria-hidden="true" />
								Créer une routine
							</Link>
						</Button>
					</>
				}
			/>
			<Notice message={notice} />
			<div className="overflow-hidden rounded-lg border bg-card">
				<RoutinesTable
					rows={rows}
					onLaunch={launchRoutine}
					onToggle={toggleRoutine}
				/>
			</div>
		</Page>
	)
}
