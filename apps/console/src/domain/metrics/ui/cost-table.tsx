import { ChevronRight } from 'lucide-react'
import { Fragment, useState } from 'react'
import { DataTable, Td, Th, Tr } from '#/domain/audit/ui/data-table'
import {
	formatCost,
	formatCount,
	formatTokens,
} from '#/domain/dashboard/format'
import type { CostRow } from '#/domain/metrics/rows'
import { cn } from '#/lib/utils'

interface Props {
	/** En-tête de la première colonne. */
	subject: string
	rows: ReadonlyArray<CostRow>
	showRuns?: boolean
}

const FIGURES = 'figures font-mono text-[11.5px]'

function ShareBar({ ratio }: { ratio: number }) {
	return (
		<div
			aria-hidden="true"
			className="h-1.5 min-w-16 overflow-hidden rounded-full bg-secondary"
		>
			<div
				className="h-full rounded-full bg-primary"
				style={{ width: `${Math.min(Math.max(ratio, 0), 1) * 100}%` }}
			/>
		</div>
	)
}

function Figures({
	row,
	showRuns,
	ratio,
}: {
	row: CostRow
	showRuns: boolean
	ratio: number
}) {
	return (
		<>
			<Td className={FIGURES}>{formatTokens(row.figures.input_tokens)}</Td>
			<Td className={FIGURES}>{formatTokens(row.figures.output_tokens)}</Td>
			{showRuns ? (
				<Td className={`${FIGURES} text-muted-foreground`}>
					{row.runs === undefined ? '' : formatCount(row.runs)}
				</Td>
			) : null}
			<Td className={FIGURES}>{formatCost(row.figures.cost_micros)}</Td>
			<Td className="w-[18%]">
				<ShareBar ratio={ratio} />
			</Td>
		</>
	)
}

export function CostTable({ subject, rows, showRuns = false }: Props) {
	const max = Math.max(1, ...rows.map((row) => row.figures.cost_micros))
	const [open, setOpen] = useState<ReadonlySet<string>>(new Set())
	const toggle = (key: string) =>
		setOpen((current) => {
			const next = new Set(current)
			if (!next.delete(key)) next.add(key)
			return next
		})
	return (
		<DataTable>
			<thead>
				<tr>
					<Th>{subject}</Th>
					<Th>Entrée</Th>
					<Th>Sortie</Th>
					{showRuns ? <Th>Exécutions</Th> : null}
					<Th>Coût</Th>
					<Th>Part</Th>
				</tr>
			</thead>
			<tbody>
				{rows.map((row) => {
					const expandable = row.details.length > 0
					const expanded = open.has(row.key)
					return (
						<Fragment key={row.key}>
							<Tr>
								<Td>
									{expandable ? (
										<button
											type="button"
											aria-expanded={expanded}
											onClick={() => toggle(row.key)}
											className="flex items-center gap-1.5 text-left text-xs focus-visible:outline-2 focus-visible:outline-ring"
										>
											<ChevronRight
												aria-hidden="true"
												className={cn(
													'size-3.5 shrink-0 transition-transform',
													expanded && 'rotate-90',
												)}
											/>
											{row.label}
										</button>
									) : (
										<span className="block pl-5 text-xs">{row.label}</span>
									)}
									{row.sub ? (
										<span className="block pl-5 text-xs text-muted-foreground">
											{row.sub}
										</span>
									) : null}
								</Td>
								<Figures
									row={row}
									showRuns={showRuns}
									ratio={row.figures.cost_micros / max}
								/>
							</Tr>
							{expanded
								? row.details.map((detail) => (
										<Tr
											key={`${row.key}/${detail.key}`}
											className="bg-secondary/40"
										>
											<Td className="pl-9 text-xs text-muted-foreground">
												{detail.label}
											</Td>
											<Figures
												row={detail}
												showRuns={showRuns}
												ratio={
													detail.figures.cost_micros /
													Math.max(1, row.figures.cost_micros)
												}
											/>
										</Tr>
									))
								: null}
						</Fragment>
					)
				})}
			</tbody>
		</DataTable>
	)
}
