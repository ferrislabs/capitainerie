import { Link } from '@tanstack/react-router'
import { Plus, Repeat, Search } from 'lucide-react'
import { useMemo, useState } from 'react'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Notice, useNotice } from '#/components/layout/notice'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { Button } from '#/components/ui/button'
import { useTeamAgents } from '#/domain/agent-team/hooks/use-team-agents'
import { useNow } from '#/domain/routines/feature/use-routine-context'
import {
	useLaunchRoutine,
	useRoutines,
	useUpdateRoutine,
} from '#/domain/routines/hooks/use-routines'
import {
	describeRoutineError,
	filterRoutines,
	type Routine,
} from '#/domain/routines/model'
import { RoutinesTable } from '#/domain/routines/ui/routines-table'
import { Empty } from '#/domain/sources/ui/mockup'

function describe(routines: ReadonlyArray<Routine>, archived: boolean): string {
	const total = routines.length
	const noun = total > 1 ? 'routines' : 'routine'
	if (archived)
		return `${total} ${noun} ${total > 1 ? 'archivées' : 'archivée'}`
	const active = routines.filter((r) => r.enabled).length
	return `Travail récurrent qui devient des tâches auditables · ${total} ${noun} · ${active} ${active > 1 ? 'actives' : 'active'}`
}

export function RoutinesPage() {
	const [showArchived, setShowArchived] = useState(false)
	const [query, setQuery] = useState('')
	const [notice, notify] = useNotice()
	const now = useNow()
	const routines = useRoutines(showArchived)
	const agents = useTeamAgents().data?.items
	const update = useUpdateRoutine()
	const launch = useLaunchRoutine()
	const items = routines.data?.items ?? []
	const agentOf = useMemo(() => {
		const byId = new Map((agents ?? []).map((a) => [a.id, a]))
		return (id: string) => byId.get(id)
	}, [agents])
	const visible = useMemo(
		() => filterRoutines(items, query, (id) => agentOf(id)?.name),
		[items, query, agentOf],
	)

	const toggle = (routine: Routine, enabled: boolean) =>
		update.mutate(
			{ path: { routine_id: routine.id }, body: { enabled } },
			{
				onSuccess: () =>
					notify(
						`${enabled ? 'Routine activée' : 'Routine en pause'} : ${routine.name}`,
					),
				onError: (error) => notify(describeRoutineError(error)),
			},
		)

	const run = (routine: Routine) =>
		launch.mutate(
			{ path: { routine_id: routine.id } },
			{
				onSuccess: () =>
					notify(
						`Routine lancée : ${routine.name}. La tâche est en cours de création.`,
					),
				onError: (error) => notify(describeRoutineError(error)),
			},
		)

	return (
		<Page>
			<PageHeader
				title="Routines"
				description={
					routines.isPending || routines.isError
						? undefined
						: describe(items, showArchived)
				}
				actions={
					<>
						<Button
							variant="outline"
							aria-pressed={showArchived}
							onClick={() => setShowArchived((value) => !value)}
						>
							{showArchived ? 'Voir les actives' : 'Voir les archivées'}
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
			<Notice message={notice} className="mb-3" />
			<label className="mb-3 flex max-w-[360px] items-center gap-2 rounded-[6px] border bg-background px-[9px] py-1.5 focus-within:border-primary">
				<Search aria-hidden="true" className="size-4 text-muted-foreground" />
				<input
					type="search"
					aria-label="Rechercher une routine"
					placeholder="Rechercher…"
					value={query}
					onChange={(event) => setQuery(event.target.value)}
					className="min-w-0 flex-1 bg-transparent text-[13px] outline-none"
				/>
			</label>
			{routines.isError ? (
				<div className="grid gap-2">
					<InlineAlert tone="danger">
						Impossible de charger les routines.
					</InlineAlert>
					<div>
						<Button variant="outline" onClick={() => void routines.refetch()}>
							Réessayer
						</Button>
					</div>
				</div>
			) : routines.isPending ? (
				<output className="m-0 text-muted-foreground">
					Chargement des routines…
				</output>
			) : (
				<div className="overflow-hidden rounded-lg border bg-card">
					{visible.length === 0 ? (
						<Empty
							title={
								query.trim()
									? 'Aucune routine ne correspond'
									: showArchived
										? 'Aucune routine archivée'
										: 'Aucune routine'
							}
						>
							{query.trim() || showArchived ? null : (
								<span className="inline-flex items-center gap-1.5">
									<Repeat aria-hidden="true" className="size-4" />
									Crée une routine pour confier un travail récurrent à un agent.
								</span>
							)}
						</Empty>
					) : (
						<RoutinesTable
							routines={visible}
							agentOf={agentOf}
							now={now}
							busy={update.isPending || launch.isPending}
							onLaunch={run}
							onToggle={toggle}
						/>
					)}
				</div>
			)}
		</Page>
	)
}
