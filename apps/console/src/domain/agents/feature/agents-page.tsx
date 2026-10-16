import { useState } from 'react'
import { Card, CardContent, CardHeader, CardTitle } from '#/components/ui/card'
import type { CostDimension } from '#/domain/agents/dimensions'
import { useAgents } from '#/domain/agents/hooks/use-agents'
import { useCosts } from '#/domain/agents/hooks/use-costs'
import { AgentsTable } from '#/domain/agents/ui/agents-table'
import { CostsSection } from '#/domain/agents/ui/costs-section'

export function AgentsPage() {
	const [by, setBy] = useState<CostDimension>('agent')
	const agents = useAgents()
	const costs = useCosts(by)

	return (
		<section className="flex flex-col gap-6">
			<h1 className="text-2xl font-semibold">Agents</h1>
			{agents.isError ? (
				<p role="alert" className="text-sm text-destructive">
					Impossible de charger les agents.
				</p>
			) : agents.isPending ? (
				<p className="text-sm text-muted-foreground">Chargement…</p>
			) : (
				<AgentsTable agents={agents.data} />
			)}
			<Card>
				<CardHeader>
					<CardTitle>Coûts des 30 derniers jours</CardTitle>
				</CardHeader>
				<CardContent>
					<CostsSection
						by={by}
						costs={costs.data}
						isPending={costs.isPending}
						isError={costs.isError}
						onChange={setBy}
					/>
				</CardContent>
			</Card>
		</section>
	)
}
