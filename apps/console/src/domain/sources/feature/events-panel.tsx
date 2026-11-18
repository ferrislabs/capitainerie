import { useState } from 'react'
import { InlineAlert } from '#/components/layout/inline-alert'
import { SectionCard } from '#/components/layout/section-card'
import { Button } from '#/components/ui/button'
import { useSourceEvents } from '#/domain/sources/hooks/use-sources'
import { EVENTS_PAGE_SIZE } from '#/domain/sources/model'
import { EventsTable } from '#/domain/sources/ui/events-table'
import { Empty } from '#/domain/sources/ui/mockup'

interface Props {
	sourceId: string
}

export function EventsPanel({ sourceId }: Props) {
	const [page, setPage] = useState(0)
	const events = useSourceEvents(
		sourceId,
		EVENTS_PAGE_SIZE,
		page * EVENTS_PAGE_SIZE,
	)
	const items = events.data?.items ?? []
	const total = events.data?.total ?? 0
	const pages = Math.max(1, Math.ceil(total / EVENTS_PAGE_SIZE))
	const first = page * EVENTS_PAGE_SIZE + 1
	const last = page * EVENTS_PAGE_SIZE + items.length

	return (
		<SectionCard
			title="Événements reçus"
			description={total > 0 ? `${total} au total` : undefined}
			flush
		>
			{events.isError ? (
				<div className="grid gap-2 p-3">
					<InlineAlert tone="danger">
						Impossible de charger les événements.
					</InlineAlert>
					<div>
						<Button variant="outline" onClick={() => void events.refetch()}>
							Réessayer
						</Button>
					</div>
				</div>
			) : events.isPending ? (
				<output className="block p-3 text-muted-foreground">
					Chargement des événements…
				</output>
			) : items.length === 0 ? (
				<Empty title="Aucun événement pour l'instant">
					Utilise « Envoyer un événement de test » pour vérifier la chaîne.
				</Empty>
			) : (
				<>
					<EventsTable events={items} />
					{pages > 1 ? (
						<nav
							aria-label="Pagination des événements"
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
