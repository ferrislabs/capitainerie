import { SectionCard } from '#/components/layout/section-card'
import { Tile } from '#/components/layout/tile'
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from '#/components/ui/table'
import type {
	AgentCost,
	CostTile,
	DailyRuns,
} from '#/domain/audit/mock/audit-data'
import { Avatar } from '#/domain/audit/ui/avatar'
import { RunsChart } from '#/domain/audit/ui/runs-chart'
import { shortName } from '#/domain/demo'

interface Props {
	tiles: ReadonlyArray<CostTile>
	days: ReadonlyArray<DailyRuns>
	agents: ReadonlyArray<AgentCost>
}

export function CostsPanel({ tiles, days, agents }: Props) {
	return (
		<>
			<div className="grid grid-cols-2 gap-3 lg:grid-cols-4">
				{tiles.map((tile) => (
					<Tile
						key={tile.label}
						label={tile.label}
						value={tile.value}
						hint={tile.hint}
					/>
				))}
			</div>
			<SectionCard title="Exécutions par jour" description="14 derniers jours">
				<RunsChart days={days} />
			</SectionCard>
			<SectionCard title="Par agent" bare>
				<div className="overflow-hidden rounded-lg border bg-card">
					<Table>
						<TableHeader>
							<TableRow>
								<TableHead>Agent</TableHead>
								<TableHead>Entrée</TableHead>
								<TableHead>Dont cache</TableHead>
								<TableHead>Sortie</TableHead>
								<TableHead>Exécutions</TableHead>
								<TableHead>Coût</TableHead>
							</TableRow>
						</TableHeader>
						<TableBody>
							{agents.map((row) => (
								<TableRow key={row.agent}>
									<TableCell>
										<span className="inline-flex items-center gap-2">
											<Avatar name={row.agent} size={18} />
											{shortName(row.agent)}
										</span>
									</TableCell>
									<TableCell className="figures">{row.input}</TableCell>
									<TableCell className="figures text-muted-foreground">
										{row.cached}
									</TableCell>
									<TableCell className="figures">{row.output}</TableCell>
									<TableCell className="figures text-muted-foreground">
										{row.runs} abo.
									</TableCell>
									<TableCell className="figures">{row.cost}</TableCell>
								</TableRow>
							))}
						</TableBody>
					</Table>
				</div>
			</SectionCard>
		</>
	)
}
