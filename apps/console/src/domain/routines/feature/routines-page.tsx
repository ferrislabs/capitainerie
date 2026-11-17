import { Link } from '@tanstack/react-router'
import { Plus } from 'lucide-react'
import { DemoBanner } from '#/components/layout/demo-banner'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { SectionCard } from '#/components/layout/section-card'
import { Button } from '#/components/ui/button'
import {
	launchRoutine,
	toggleRoutine,
	useRoutinesState,
} from '#/domain/routines/feature/routines-store'
import { useNotice } from '#/domain/routines/feature/use-routine-context'
import { scheduleLabel } from '#/domain/routines/routine-view'
import { RoutineNotice } from '#/domain/routines/ui/routine-notice'
import { RoutinesTable } from '#/domain/routines/ui/routines-table'

export function RoutinesPage() {
	const routines = useRoutinesState((s) => s.routines)
	const notice = useNotice()
	const rows = routines.map((routine) => ({
		routine,
		scheduleLabel: scheduleLabel(routine),
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
							<Link to="/runs">Toutes les exécutions</Link>
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
			<RoutineNotice message={notice} />
			<SectionCard title="Routines" bare headingLevel="h2">
				<RoutinesTable
					rows={rows}
					onLaunch={launchRoutine}
					onToggle={toggleRoutine}
				/>
			</SectionCard>
		</Page>
	)
}
