import { KanbanView } from '#/domain/kanban/pages/ui/kanban-view'

export function KanbanPage() {
	return (
		<KanbanView
			heading="Kanban"
			emptyTitle="Aucune carte"
			emptyDescription="Les cartes du tableau apparaîtront ici."
		/>
	)
}
