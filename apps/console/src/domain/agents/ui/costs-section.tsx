import { Coins } from 'lucide-react'
import type { Schemas } from '#/api/api.client'
import { EmptyState } from '#/components/layout/empty-state'
import { Button } from '#/components/ui/button'
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

interface Props {
	by: CostDimension
	costs: Schemas.CostsResponse | undefined
	isPending: boolean
	isError: boolean
	onChange: (by: CostDimension) => void
}

export function CostsSection({
	by,
	costs,
	isPending,
	isError,
	onChange,
}: Props) {
	const keyLabel = DIMENSIONS.find((d) => d.value === by)?.keyLabel ?? ''

	return (
		<div className="flex flex-col gap-4">
			<fieldset className="flex gap-2">
				<legend className="sr-only">Regrouper les coûts</legend>
				{DIMENSIONS.map((dimension) => (
					<Button
						key={dimension.value}
						type="button"
						size="sm"
						variant={dimension.value === by ? 'default' : 'outline'}
						aria-pressed={dimension.value === by}
						onClick={() => onChange(dimension.value)}
					>
						{dimension.label}
					</Button>
				))}
			</fieldset>
			{isError ? (
				<p role="alert" className="text-sm text-destructive">
					Impossible de charger les coûts.
				</p>
			) : isPending || !costs ? (
				<p className="text-sm text-muted-foreground">Chargement…</p>
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
								<TableCell className="font-medium">{row.key}</TableCell>
								<TableCell className="text-right">{row.runs}</TableCell>
								<TableCell className="text-right">
									{formatTokens(row.input_tokens)} /{' '}
									{formatTokens(row.output_tokens)}
								</TableCell>
								<TableCell className="text-right">
									{row.latency_ms_p50 == null
										? '—'
										: formatLatency(row.latency_ms_p50)}
								</TableCell>
								<TableCell className="text-right">
									{formatCost(row.cost_micros)}
								</TableCell>
							</TableRow>
						))}
						<TableRow className="font-semibold">
							<TableCell>Total</TableCell>
							<TableCell />
							<TableCell className="text-right">
								{formatTokens(costs.total.input_tokens)} /{' '}
								{formatTokens(costs.total.output_tokens)}
							</TableCell>
							<TableCell />
							<TableCell className="text-right">
								{formatCost(costs.total.cost_micros)}
							</TableCell>
						</TableRow>
					</TableBody>
				</Table>
			)}
		</div>
	)
}
