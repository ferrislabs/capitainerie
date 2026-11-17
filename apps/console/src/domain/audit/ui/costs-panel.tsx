import { Tile } from '#/components/layout/tile'
import type { AgentCost, CostTile } from '#/domain/audit/mock/audit-data'
import { AgentAvatar } from '#/domain/audit/ui/avatar'
import { DataTable, Td, Th, Tr } from '#/domain/audit/ui/data-table'
import { Eyebrow } from '#/domain/audit/ui/rows'
import { shortName } from '#/domain/demo'

interface Props {
	tiles: ReadonlyArray<CostTile>
	agents: ReadonlyArray<AgentCost>
}

const FIGURES = 'figures font-mono text-[11.5px]'

export function CostsPanel({ tiles, agents }: Props) {
	return (
		<>
			<div className="grid grid-cols-2 gap-2.5 lg:grid-cols-4">
				{tiles.map((tile) => (
					<Tile
						key={tile.label}
						label={tile.label}
						value={tile.value}
						hint={tile.hint}
					/>
				))}
			</div>
			<Eyebrow>Par agent</Eyebrow>
			<DataTable>
				<thead>
					<tr>
						<Th>Agent</Th>
						<Th>Entrée</Th>
						<Th>Dont cache</Th>
						<Th>Sortie</Th>
						<Th>Exécutions</Th>
						<Th>Coût</Th>
					</tr>
				</thead>
				<tbody>
					{agents.map((row) => (
						<Tr key={row.agent}>
							<Td>
								<span className="flex items-center gap-1.5 text-xs whitespace-nowrap">
									<AgentAvatar name={row.agent} size={18} />
									{shortName(row.agent)}
								</span>
							</Td>
							<Td className={FIGURES}>{row.input}</Td>
							<Td className={`${FIGURES} text-muted-foreground`}>
								{row.cached}
							</Td>
							<Td className={FIGURES}>{row.output}</Td>
							<Td className={`${FIGURES} text-muted-foreground`}>
								{row.runs} abo.
							</Td>
							<Td className={FIGURES}>{row.cost}</Td>
						</Tr>
					))}
				</tbody>
			</DataTable>
		</>
	)
}
