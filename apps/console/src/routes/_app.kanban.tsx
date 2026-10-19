import { createFileRoute } from '@tanstack/react-router'
import { parseCardSearch } from '#/domain/kanban/card-search'
import { KanbanPage } from '#/domain/kanban/feature/kanban-page'

export const Route = createFileRoute('/_app/kanban')({
	validateSearch: parseCardSearch,
	component: KanbanRoute,
})

function KanbanRoute() {
	const { card } = Route.useSearch()
	const navigate = Route.useNavigate()
	return (
		<KanbanPage
			openCardId={card}
			onOpenCard={(cardId) =>
				void navigate({
					search: { card: cardId },
					replace: cardId === undefined,
				})
			}
		/>
	)
}
