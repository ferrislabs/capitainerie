import { Link } from '@tanstack/react-router'
import { Badge } from '#/components/ui/badge'
import {
	Sheet,
	SheetBody,
	SheetContent,
	SheetDescription,
	SheetHeader,
	SheetTitle,
} from '#/components/ui/sheet'
import { agentRole } from '#/domain/agents/catalog'
import { type AgentRow, modelLabel } from '#/domain/agents/filters'
import { formatCost, formatTokens } from '#/domain/runs/format'
import { EXECUTOR_INFO, ROLE_INFO } from '#/domain/settings/roles'

const dateFormat = new Intl.DateTimeFormat('fr-FR', {
	dateStyle: 'short',
	timeStyle: 'short',
})

interface Props {
	row: AgentRow | undefined
	onClose: () => void
}

function roleLabel(id: string): string | undefined {
	const role = agentRole(id)
	if (!role) return undefined
	return role === 'executor' ? EXECUTOR_INFO.label : ROLE_INFO[role].label
}

function Fact({
	label,
	children,
}: {
	label: string
	children: React.ReactNode
}) {
	return (
		<div className="flex items-baseline justify-between gap-4 border-b py-2 text-[13px] last:border-b-0">
			<dt className="text-muted-foreground">{label}</dt>
			<dd className="figures text-right">{children}</dd>
		</div>
	)
}

export function AgentSheet({ row, onClose }: Props) {
	return (
		<Sheet open={row !== undefined} onOpenChange={(open) => !open && onClose()}>
			<SheetContent>
				{row ? (
					<>
						<SheetHeader>
							<SheetTitle>{row.name}</SheetTitle>
							<SheetDescription>
								{row.description ?? 'Agent absent du catalogue de la console.'}
							</SheetDescription>
							{row.source ? (
								<Badge variant="neutral">{row.source}</Badge>
							) : null}
						</SheetHeader>
						<SheetBody>
							<section aria-labelledby="agent-config">
								<h3 id="agent-config" className="text-[15px] font-semibold">
									Configuration
								</h3>
								<dl className="mt-2">
									<Fact label="Rôle">{roleLabel(row.agent.agent) ?? '—'}</Fact>
									<Fact label="Modèle">
										<span className="font-mono text-xs">{modelLabel(row)}</span>
									</Fact>
									<Fact label="Identifiant">
										<span className="font-mono text-xs break-all">
											{row.agent.agent}
										</span>
									</Fact>
								</dl>
							</section>
							<section aria-labelledby="agent-stats">
								<h3 id="agent-stats" className="text-[15px] font-semibold">
									Statistiques
								</h3>
								<dl className="mt-2">
									<Fact label="Runs">{row.agent.runs}</Fact>
									<Fact label="Dernière activité">
										{dateFormat.format(new Date(row.agent.last_seen))}
									</Fact>
									<Fact label="Tokens en entrée">
										{formatTokens(row.agent.input_tokens)}
									</Fact>
									<Fact label="Tokens en sortie">
										{formatTokens(row.agent.output_tokens)}
									</Fact>
									<Fact label="Coût">{formatCost(row.agent.cost_micros)}</Fact>
								</dl>
							</section>
							<section aria-labelledby="agent-runs">
								<h3 id="agent-runs" className="text-[15px] font-semibold">
									Derniers runs
								</h3>
								<p className="mt-2 text-[13px] text-muted-foreground">
									Pas encore disponible : l’API ne filtre pas les runs par
									agent.{' '}
									<Link to="/runs" className="text-link underline">
										Voir tous les runs
									</Link>
								</p>
							</section>
						</SheetBody>
					</>
				) : null}
			</SheetContent>
		</Sheet>
	)
}
