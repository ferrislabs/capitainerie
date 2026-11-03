import { Badge } from '#/components/ui/badge'
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from '#/components/ui/table'
import { type AgentRow, modelLabel } from '#/domain/agents/filters'
import { CopyIdButton } from '#/domain/agents/ui/copy-id-button'
import { formatCost } from '#/domain/runs/format'

const dateFormat = new Intl.DateTimeFormat('fr-FR', {
	dateStyle: 'short',
	timeStyle: 'short',
})

interface Props {
	rows: ReadonlyArray<AgentRow>
	onOpen: (row: AgentRow) => void
}

export function AgentsTable({ rows, onOpen }: Props) {
	return (
		<Table>
			<TableHeader>
				<TableRow>
					<TableHead>Agent</TableHead>
					<TableHead>Description</TableHead>
					<TableHead>Source</TableHead>
					<TableHead>Modèle</TableHead>
					<TableHead className="text-right">Runs</TableHead>
					<TableHead>Dernière activité</TableHead>
					<TableHead className="text-right">Coût</TableHead>
					<TableHead>Identifiant</TableHead>
				</TableRow>
			</TableHeader>
			<TableBody>
				{rows.map((row) => (
					<TableRow
						key={row.agent.agent}
						onClick={() => onOpen(row)}
						className="cursor-pointer"
					>
						<TableCell className="whitespace-nowrap">
							<button
								type="button"
								onClick={(event) => {
									event.stopPropagation()
									onOpen(row)
								}}
								className="rounded-sm text-left font-medium hover:text-link hover:underline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring"
							>
								{row.name}
							</button>
						</TableCell>
						<TableCell className="max-w-80 min-w-56 text-muted-foreground">
							{row.description ?? '—'}
						</TableCell>
						<TableCell>
							{row.source ? <Badge variant="neutral">{row.source}</Badge> : '—'}
						</TableCell>
						<TableCell className="whitespace-nowrap font-mono text-xs">
							{modelLabel(row)}
						</TableCell>
						<TableCell className="text-right figures">
							{row.agent.runs}
						</TableCell>
						<TableCell className="whitespace-nowrap text-muted-foreground figures">
							{dateFormat.format(new Date(row.agent.last_seen))}
						</TableCell>
						<TableCell className="whitespace-nowrap text-right figures">
							{formatCost(row.agent.cost_micros)}
						</TableCell>
						<TableCell>
							<div className="flex items-center gap-1">
								<span className="font-mono text-xs whitespace-nowrap">
									{row.agent.agent}
								</span>
								<CopyIdButton value={row.agent.agent} />
							</div>
						</TableCell>
					</TableRow>
				))}
			</TableBody>
		</Table>
	)
}
