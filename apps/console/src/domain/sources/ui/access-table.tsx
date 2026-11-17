import { Badge } from '#/components/ui/badge'
import { Button } from '#/components/ui/button'
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from '#/components/ui/table'
import type { DemoAgent } from '#/domain/demo'
import type { AccessRow } from '#/domain/sources/access'
import { AgentAvatar } from '#/domain/sources/ui/agent-avatar'

interface Props {
	rows: ReadonlyArray<AccessRow>
	agents: ReadonlyArray<DemoAgent>
	grantee: { id: string; label: string } | null
	onGrant: (agentId: string, connector: string) => void
}

export function AccessTable({ rows, agents, grantee, onGrant }: Props) {
	return (
		<div>
			<Table>
				<TableHeader>
					<TableRow>
						<TableHead>Connecteur</TableHead>
						<TableHead>Étape</TableHead>
						<TableHead>Agents autorisés</TableHead>
						<TableHead />
					</TableRow>
				</TableHeader>
				<TableBody>
					{rows.map((row) => {
						const granted = row.agentIds
							.map((id) => agents.find((a) => a.id === id))
							.filter((a): a is DemoAgent => a !== undefined)
						return (
							<TableRow key={row.connector}>
								<TableCell>{row.connector}</TableCell>
								<TableCell className="text-muted-foreground">
									{row.step}
								</TableCell>
								<TableCell>
									{!row.installed ? (
										<Badge variant="danger">Connecteur non installé</Badge>
									) : granted.length > 0 ? (
										<span className="flex -space-x-1">
											{granted.map((a) => (
												<AgentAvatar key={a.id} agent={a} />
											))}
										</span>
									) : (
										<Badge variant="danger">Aucun agent autorisé</Badge>
									)}
								</TableCell>
								<TableCell className="text-right">
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
								</TableCell>
							</TableRow>
						)
					})}
				</TableBody>
			</Table>
			<p className="px-3 py-2 text-muted-foreground">
				Un agent ne peut utiliser que les connecteurs que tu lui as autorisés.
				Le pipeline s'arrête sur une étape sans accès.
			</p>
		</div>
	)
}
