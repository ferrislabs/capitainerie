import { Bot } from 'lucide-react'
import type { Schemas } from '#/api/api.client'
import { EmptyState } from '#/components/layout/empty-state'
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from '#/components/ui/table'
import { formatCost, formatTokens } from '#/domain/runs/format'

const dateFormat = new Intl.DateTimeFormat('fr-FR', {
	dateStyle: 'short',
	timeStyle: 'short',
})

interface Props {
	agents: ReadonlyArray<Schemas.AgentResponse>
}

export function AgentsTable({ agents }: Props) {
	if (agents.length === 0) {
		return (
			<EmptyState
				icon={Bot}
				title="Aucun agent"
				description="Les agents apparaîtront ici après leur premier run."
				className="m-4"
			/>
		)
	}

	return (
		<Table>
			<TableHeader>
				<TableRow>
					<TableHead>Agent</TableHead>
					<TableHead className="text-right">Runs</TableHead>
					<TableHead>Dernière activité</TableHead>
					<TableHead className="text-right">Tokens entrée/sortie</TableHead>
					<TableHead className="text-right">Coût</TableHead>
				</TableRow>
			</TableHeader>
			<TableBody>
				{agents.map((agent) => (
					<TableRow key={agent.agent}>
						<TableCell className="font-mono text-xs font-medium">
							{agent.agent}
						</TableCell>
						<TableCell className="text-right figures">{agent.runs}</TableCell>
						<TableCell className="whitespace-nowrap text-muted-foreground figures">
							{dateFormat.format(new Date(agent.last_seen))}
						</TableCell>
						<TableCell className="text-right figures">
							{formatTokens(agent.input_tokens)} /{' '}
							{formatTokens(agent.output_tokens)}
						</TableCell>
						<TableCell className="text-right figures">
							{formatCost(agent.cost_micros)}
						</TableCell>
					</TableRow>
				))}
			</TableBody>
		</Table>
	)
}
