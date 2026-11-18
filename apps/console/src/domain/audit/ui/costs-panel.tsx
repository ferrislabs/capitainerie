import type { Schemas } from '#/api/api.client'
import { Tile } from '#/components/layout/tile'
import { formatCents } from '#/domain/audit/model'
import { DataTable, Td, Th, Tr } from '#/domain/audit/ui/data-table'
import { Eyebrow, Panel, PanelEmpty } from '#/domain/audit/ui/rows'
import {
	formatCost,
	formatCount,
	formatTokens,
} from '#/domain/dashboard/format'
import { AgentAvatar } from '#/domain/tasks/ui/agent-chip'
import { shortName } from '#/lib/agent-name'

interface Props {
	costs: Schemas.CostsResponse
	budgets: Schemas.BudgetsResponse | undefined
	agentName: (key: string) => string
	hueOf: (id: string | null | undefined, name: string) => number
}

const FIGURES = 'figures font-mono text-[11.5px]'

export function CostsPanel({ costs, budgets, agentName, hueOf }: Props) {
	const { total, rows } = costs
	const runs = rows.reduce((sum, row) => sum + row.runs, 0)
	const organization = budgets?.items.find(
		(budget) => budget.scope === 'organization',
	)
	return (
		<>
			<div className="grid grid-cols-2 gap-2.5 lg:grid-cols-4">
				<Tile
					label="Dépense du mois"
					value={formatCost(total.cost_micros)}
					hint={`${formatCount(runs)} ${runs > 1 ? 'exécutions' : 'exécution'}`}
				/>
				<Tile
					label="Jetons en entrée"
					value={formatTokens(total.input_tokens)}
					hint="ce mois"
				/>
				<Tile
					label="Jetons en sortie"
					value={formatTokens(total.output_tokens)}
					hint="ce mois"
				/>
				<Tile
					label="Budget"
					value={organization ? `${organization.percent} %` : '–'}
					hint={
						organization
							? `sur ${formatCents(organization.monthly_limit_cents)} ce mois`
							: "Pas de budget d'organisation"
					}
					tone={
						organization?.alert || organization?.exceeded ? 'warn' : 'default'
					}
				/>
			</div>
			<Eyebrow>Par agent</Eyebrow>
			{rows.length === 0 ? (
				<Panel>
					<PanelEmpty title="Aucune dépense ce mois">
						Les coûts apparaissent après les premières exécutions.
					</PanelEmpty>
				</Panel>
			) : (
				<DataTable>
					<thead>
						<tr>
							<Th>Agent</Th>
							<Th>Entrée</Th>
							<Th>Sortie</Th>
							<Th>Exécutions</Th>
							<Th>Coût</Th>
						</tr>
					</thead>
					<tbody>
						{rows.map((row) => {
							const name = agentName(row.key)
							return (
								<Tr key={row.key}>
									<Td>
										<span className="flex items-center gap-1.5 text-xs whitespace-nowrap">
											<AgentAvatar hue={hueOf(row.key, name)} size={18} />
											{shortName(name)}
										</span>
									</Td>
									<Td className={FIGURES}>{formatTokens(row.input_tokens)}</Td>
									<Td className={FIGURES}>{formatTokens(row.output_tokens)}</Td>
									<Td className={`${FIGURES} text-muted-foreground`}>
										{formatCount(row.runs)}
									</Td>
									<Td className={FIGURES}>{formatCost(row.cost_micros)}</Td>
								</Tr>
							)
						})}
					</tbody>
				</DataTable>
			)}
		</>
	)
}
