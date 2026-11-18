import { Combobox } from '#/components/ui/combobox'
import { NativeSelect } from '#/components/ui/native-select'

export interface FilterOption {
	id: string
	name: string
}

interface Props {
	agents: ReadonlyArray<FilterOption>
	tasks: ReadonlyArray<FilterOption>
	agentId: string
	cardId: string
	onAgent: (id: string) => void
	onCard: (id: string) => void
}

export function MetricsFilters({
	agents,
	tasks,
	agentId,
	cardId,
	onAgent,
	onCard,
}: Props) {
	return (
		<>
			<NativeSelect
				aria-label="Filtrer par agent"
				value={agentId}
				onChange={(e) => onAgent(e.target.value)}
				className="w-auto"
			>
				<option value="">Tous les agents</option>
				{agents.map((agent) => (
					<option key={agent.id} value={agent.id}>
						{agent.name}
					</option>
				))}
			</NativeSelect>
			<Combobox
				aria-label="Filtrer par tâche"
				value={cardId}
				onValueChange={onCard}
				items={tasks.map((task) => ({ value: task.id, label: task.name }))}
				placeholder="Toutes les tâches"
				emptyText="Aucune tâche ne correspond"
				clearable
				searchable
				className="w-60"
			/>
		</>
	)
}
