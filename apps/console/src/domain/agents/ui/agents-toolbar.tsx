import { Search } from 'lucide-react'
import { FilterChip } from '#/components/layout/filter-chip'
import { Input } from '#/components/ui/input'
import type { AgentFilters } from '#/domain/agents/filters'

interface Props {
	filters: AgentFilters
	sources: ReadonlyArray<string>
	models: ReadonlyArray<string>
	onChange: (filters: AgentFilters) => void
}

export function AgentsToolbar({ filters, sources, models, onChange }: Props) {
	return (
		<search
			aria-label="Filtrer les agents"
			className="flex flex-wrap items-center gap-2"
		>
			<FilterChip
				label="Source"
				options={sources}
				selected={filters.sources}
				onChange={(values) => onChange({ ...filters, sources: values })}
			/>
			<FilterChip
				label="Modèle"
				options={models}
				selected={filters.models}
				onChange={(values) => onChange({ ...filters, models: values })}
			/>
			<Input
				type="search"
				aria-label="Rechercher un agent"
				placeholder="Rechercher un agent"
				value={filters.query}
				onChange={(event) =>
					onChange({ ...filters, query: event.target.value })
				}
				leading={<Search aria-hidden="true" />}
				clearable
				wrapperClassName="min-w-48 flex-1"
			/>
		</search>
	)
}
