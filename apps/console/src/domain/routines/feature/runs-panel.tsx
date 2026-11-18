import { useState } from 'react'
import { InlineAlert } from '#/components/layout/inline-alert'
import { SectionCard } from '#/components/layout/section-card'
import { Button } from '#/components/ui/button'
import { useRoutineRuns } from '#/domain/routines/hooks/use-routines'
import { RUNS_PAGE_SIZE } from '#/domain/routines/model'
import { RunsTable } from '#/domain/routines/ui/runs-table'
import { Empty } from '#/domain/sources/ui/mockup'

interface Props {
	routineId: string
	now: number
}

export function RunsPanel({ routineId, now }: Props) {
	const [page, setPage] = useState(0)
	const runs = useRoutineRuns(routineId, RUNS_PAGE_SIZE, page * RUNS_PAGE_SIZE)
	const items = runs.data?.items ?? []
	const total = runs.data?.total ?? 0
	const pages = Math.max(1, Math.ceil(total / RUNS_PAGE_SIZE))
	const first = page * RUNS_PAGE_SIZE + 1
	const last = page * RUNS_PAGE_SIZE + items.length

	return (
		<SectionCard
			title="Passages"
			description={total > 0 ? `${total} au total` : undefined}
			flush
		>
			{runs.isError ? (
				<div className="grid gap-2 p-3">
					<InlineAlert tone="danger">
						Impossible de charger les passages.
					</InlineAlert>
					<div>
						<Button variant="outline" onClick={() => void runs.refetch()}>
							Réessayer
						</Button>
					</div>
				</div>
			) : runs.isPending ? (
				<output className="block p-3 text-muted-foreground">
					Chargement des passages…
				</output>
			) : items.length === 0 ? (
				<Empty title="Jamais lancée">
					Utilise « Lancer maintenant » pour créer une première tâche.
				</Empty>
			) : (
				<>
					<RunsTable runs={items} now={now} />
					{pages > 1 ? (
						<nav
							aria-label="Pagination des passages"
							className="flex items-center justify-end gap-2 border-t px-3 py-2 text-[12px] text-muted-foreground"
						>
							<span>{`${first}–${last} sur ${total}`}</span>
							<Button
								type="button"
								size="sm"
								variant="outline"
								disabled={page === 0}
								onClick={() => setPage((p) => Math.max(0, p - 1))}
							>
								Précédents
							</Button>
							<Button
								type="button"
								size="sm"
								variant="outline"
								disabled={page + 1 >= pages}
								onClick={() => setPage((p) => p + 1)}
							>
								Suivants
							</Button>
						</nav>
					) : null}
				</>
			)}
		</SectionCard>
	)
}
