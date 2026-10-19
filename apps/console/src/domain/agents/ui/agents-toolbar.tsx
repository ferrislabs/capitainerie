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
			<div className="relative min-w-48 flex-1">
				<Search
					aria-hidden="true"
					className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground"
				/>
				<Input
					type="search"
					aria-label="Rechercher un agent"
					placeholder="Rechercher un agent"
					value={filters.query}
					onChange={(event) =>
						onChange({ ...filters, query: event.target.value })
					}
					className="pl-8"
				/>
			</div>
		</search>
	)
}
