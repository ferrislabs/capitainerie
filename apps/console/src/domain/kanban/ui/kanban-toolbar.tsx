import { FilterChip } from '#/components/layout/filter-chip'
import type { CardFilters } from '#/domain/kanban/card-filters'

interface Props {
	filters: CardFilters
	clients: ReadonlyArray<string>
	projects: ReadonlyArray<string>
	onChange: (filters: CardFilters) => void
}

export function KanbanToolbar({ filters, clients, projects, onChange }: Props) {
	return (
		<search
			aria-label="Filtrer les cartes"
			className="flex flex-wrap items-center gap-2"
		>
			<FilterChip
				label="Client"
				options={clients}
				selected={filters.clients}
				onChange={(values) => onChange({ ...filters, clients: values })}
			/>
			<FilterChip
				label="Projet"
				options={projects}
				selected={filters.projects}
				onChange={(values) => onChange({ ...filters, projects: values })}
			/>
		</search>
	)
}
