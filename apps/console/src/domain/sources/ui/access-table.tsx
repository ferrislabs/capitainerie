import { Button } from '#/components/ui/button'
import type { DemoAgent } from '#/domain/demo'
import type { AccessRow } from '#/domain/sources/access'
import { AgentAvatar } from '#/domain/sources/ui/agent-avatar'
import { Sev, Tbl, Td, Th, Tr } from '#/domain/sources/ui/mockup'

interface Props {
	rows: ReadonlyArray<AccessRow>
	agents: ReadonlyArray<DemoAgent>
	grantee: { id: string; label: string } | null
	onGrant: (agentId: string, connector: string) => void
}

export function AccessTable({ rows, agents, grantee, onGrant }: Props) {
	return (
		<div>
			<Tbl>
				<thead>
					<tr>
						<Th>Connecteur</Th>
						<Th>Étape</Th>
						<Th>Agents autorisés</Th>
						<Th />
					</tr>
				</thead>
				<tbody>
					{rows.map((row) => {
						const granted = row.agentIds
							.map((id) => agents.find((a) => a.id === id))
							.filter((a): a is DemoAgent => a !== undefined)
						return (
							<Tr key={row.connector}>
								<Td>{row.connector}</Td>
								<Td className="text-muted-foreground">{row.step}</Td>
								<Td>
									{!row.installed ? (
										<Sev tone="crit">Connecteur non installé</Sev>
									) : granted.length > 0 ? (
										<span className="flex min-w-0 items-center gap-1.5 text-[12px]">
											{granted.map((a) => (
												<AgentAvatar key={a.id} agent={a} />
											))}
										</span>
									) : (
										<Sev tone="crit">Aucun agent autorisé</Sev>
									)}
								</Td>
								<Td className="text-right">
									{row.installed && granted.length === 0 && grantee ? (
										<Button
											type="button"
											size="sm"
											variant="outline"
											onClick={() => onGrant(grantee.id, row.connector)}
										>
											Autoriser {grantee.label}
										</Button>
									) : null}
								</Td>
							</Tr>
						)
					})}
				</tbody>
			</Tbl>
			<p className="m-0 px-3 pt-2 pb-3 text-muted-foreground">
				Un agent ne peut utiliser que les connecteurs que tu lui as autorisés.
				Le pipeline s'arrête sur une étape sans accès.
			</p>
		</div>
	)
}
