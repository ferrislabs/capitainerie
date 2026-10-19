import { createFileRoute } from '@tanstack/react-router'
import { parseCardSearch } from '#/domain/kanban/card-search'
import { KanbanPage } from '#/domain/kanban/feature/kanban-page'

export const Route = createFileRoute('/_app/kanban')({
	validateSearch: parseCardSearch,
	component: KanbanRoute,
})

function KanbanRoute() {
	const { card, client, project } = Route.useSearch()
	const navigate = Route.useNavigate()
	return (
		<KanbanPage
			openCardId={card}
			onOpenCard={(cardId) =>
				void navigate({
					search: (previous) => ({ ...previous, card: cardId }),
					replace: cardId === undefined,
				})
			}
			filters={{ clients: client ?? [], projects: project ?? [] }}
			onFiltersChange={(filters) =>
				void navigate({
					search: (previous) => ({
						...previous,
						client:
							filters.clients.length > 0 ? [...filters.clients] : undefined,
						project:
							filters.projects.length > 0 ? [...filters.projects] : undefined,
					}),
					replace: true,
				})
			}
		/>
	)
}
