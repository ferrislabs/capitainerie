import { Coins } from 'lucide-react'
import type { Schemas } from '#/api/api.client'
import { EmptyState } from '#/components/layout/empty-state'
import { InlineAlert } from '#/components/layout/inline-alert'
import { SectionCard } from '#/components/layout/section-card'
import { TableSkeleton } from '#/components/layout/table-skeleton'
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from '#/components/ui/table'
import { type CostDimension, DIMENSIONS } from '#/domain/agents/dimensions'
import { formatCost, formatLatency, formatTokens } from '#/domain/runs/format'
import { cn } from '#/lib/utils'

interface Props {
	title?: string
	by: CostDimension
	costs: Schemas.CostsResponse | undefined
	isPending: boolean
	isError: boolean
	onChange: (by: CostDimension) => void
}

export function CostsSection({
	title = 'Coûts',
	by,
	costs,
	isPending,
	isError,
	onChange,
}: Props) {
	const keyLabel = DIMENSIONS.find((d) => d.value === by)?.keyLabel ?? ''
	const identifierKey = by === 'agent' || by === 'model'

	return (
		<SectionCard
			title={title}
			description="Étapes terminées, regroupées au choix."
			bare
			actions={
				<fieldset className="flex items-center rounded-md border bg-muted/60 p-0.5">
					<legend className="sr-only">Regrouper les coûts</legend>
					{DIMENSIONS.map((dimension) => {
						const active = dimension.value === by
						return (
							<button
								key={dimension.value}
								type="button"
								aria-pressed={active}
								onClick={() => onChange(dimension.value)}
								className={cn(
									'h-6 rounded-sm px-2.5 text-xs font-medium whitespace-nowrap text-muted-foreground transition-colors duration-150 hover:text-foreground',
									'focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-ring',
									active && 'bg-card text-foreground ring-1 ring-border',
								)}
							>
								{dimension.label}
							</button>
						)
					})}
				</fieldset>
			}
		>
			{isError ? (
				<InlineAlert tone="danger">
					Impossible de charger les coûts.
				</InlineAlert>
			) : isPending || !costs ? (
				<TableSkeleton rows={4} columns={5} />
			) : costs.rows.length === 0 ? (
				<EmptyState
					icon={Coins}
					title="Aucun coût"
					description="Aucune étape terminée sur les 30 derniers jours."
				/>
			) : (
				<Table>
					<TableHeader>
						<TableRow>
							<TableHead>{keyLabel}</TableHead>
							<TableHead className="text-right">Runs</TableHead>
							<TableHead className="text-right">Tokens entrée/sortie</TableHead>
							<TableHead className="text-right">Latence médiane</TableHead>
							<TableHead className="text-right">Coût</TableHead>
						</TableRow>
					</TableHeader>
					<TableBody>
						{costs.rows.map((row) => (
							<TableRow key={row.key}>
								<TableCell
									className={cn(
										'font-medium',
										identifierKey && 'font-mono text-xs',
									)}
								>
									{row.key === 'unknown' ? 'non attribué' : row.key}
								</TableCell>
								<TableCell className="text-right figures">{row.runs}</TableCell>
								<TableCell className="text-right figures">
									{formatTokens(row.input_tokens)} /{' '}
									{formatTokens(row.output_tokens)}
								</TableCell>
								<TableCell className="text-right figures">
									{row.latency_ms_p50 == null
										? '—'
										: formatLatency(row.latency_ms_p50)}
								</TableCell>
								<TableCell className="text-right figures">
									{formatCost(row.cost_micros)}
								</TableCell>
							</TableRow>
						))}
						<TableRow className="bg-muted/50 font-semibold">
							<TableCell>Total</TableCell>
							<TableCell />
							<TableCell className="text-right figures">
								{formatTokens(costs.total.input_tokens)} /{' '}
								{formatTokens(costs.total.output_tokens)}
							</TableCell>
							<TableCell />
							<TableCell className="text-right figures">
								{formatCost(costs.total.cost_micros)}
							</TableCell>
						</TableRow>
					</TableBody>
				</Table>
			)}
		</SectionCard>
	)
}
