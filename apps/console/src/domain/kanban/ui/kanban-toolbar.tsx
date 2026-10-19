import { FilterChip } from '#/components/layout/filter-chip'
import type { CardFilters, FilterOptions } from '#/domain/kanban/card-filters'

interface Props {
	filters: CardFilters
	options: FilterOptions
	assigneeLabels?: ReadonlyMap<string, string>
	onChange: (filters: CardFilters) => void
}

export function KanbanToolbar({
	filters,
	options,
	assigneeLabels,
	onChange,
}: Props) {
	return (
		<search
			aria-label="Filtrer les cartes"
			className="flex flex-wrap items-center gap-2"
		>
			<FilterChip
				label="Client"
				options={options.clients}
				selected={filters.clients}
				onChange={(values) => onChange({ ...filters, clients: values })}
			/>
			<FilterChip
				label="Projet"
				options={options.projects}
				selected={filters.projects}
				onChange={(values) => onChange({ ...filters, projects: values })}
			/>
			<FilterChip
				label="Priorité"
				options={options.priorities}
				selected={filters.priorities}
				onChange={(values) => onChange({ ...filters, priorities: values })}
			/>
			<FilterChip
				label="Responsable"
				options={options.assignees}
				labels={assigneeLabels}
				selected={filters.assignees}
				onChange={(values) => onChange({ ...filters, assignees: values })}
			/>
			<FilterChip
				label="Étiquette"
				options={options.labels}
				selected={filters.labels}
				onChange={(values) => onChange({ ...filters, labels: values })}
			/>
		</search>
	)
}
