import { Link } from '@tanstack/react-router'
import { Plus, X } from 'lucide-react'
import { useState } from 'react'
import { Badge } from '#/components/ui/badge'
import { Button } from '#/components/ui/button'
import { NativeSelect } from '#/components/ui/native-select'
import { type DemoAgent, type DemoConnector, shortName } from '#/domain/demo'

interface Props {
	connector: DemoConnector
	agents: ReadonlyArray<DemoAgent>
	grantedIds: ReadonlyArray<string>
	onAddAccount: (connector: string) => void
	onGrant: (agentId: string, connector: string) => void
	onRevoke: (agentId: string, connector: string) => void
}

export function ConnectorCard({
	connector,
	agents,
	grantedIds,
	onAddAccount,
	onGrant,
	onRevoke,
}: Props) {
	const granted = agents.filter((a) => grantedIds.includes(a.id))
	const candidates = agents.filter((a) => !grantedIds.includes(a.id))
	const [picked, setPicked] = useState('')
	const selected = candidates.some((a) => a.id === picked)
		? picked
		: (candidates[0]?.id ?? '')
	const toolCount = connector.tools.length
	const hasAccount = connector.accounts.length > 0

	return (
		<article className="flex flex-col gap-2 rounded-lg border bg-card p-3">
			<div className="flex items-start gap-2">
				<div className="min-w-0 flex-1">
					<h3 className="flex items-center gap-2 font-medium">
						{connector.name}
						<Badge variant="muted">{connector.kind}</Badge>
					</h3>
					<p className="text-muted-foreground">{connector.description}</p>
				</div>
				<Button
					variant="outline"
					size="sm"
					onClick={() => onAddAccount(connector.name)}
				>
					<Plus aria-hidden="true" />
					Compte
				</Button>
			</div>
			{hasAccount ? (
				connector.accounts.map((account) => (
					<div key={account} className="flex items-center gap-2 border-t pt-2">
						<Badge variant="success">Connecté</Badge>
						<span className="min-w-0 flex-1 truncate">{account}</span>
					</div>
				))
			) : (
				<div className="border-t pt-2 text-muted-foreground">
					Aucun compte connecté
				</div>
			)}
			<div className="flex flex-col gap-2 border-t pt-2">
				<div className="text-muted-foreground">
					{toolCount} outil{toolCount > 1 ? 's' : ''} · agents autorisés
				</div>
				<div className="flex flex-wrap gap-1.5">
					{granted.length === 0 ? (
						<span className="text-muted-foreground">
							Aucun : accès refusé à tous les agents
						</span>
					) : (
						granted.map((a) => (
							<span
								key={a.id}
								className="inline-flex items-center gap-1 rounded-md border bg-muted px-1.5 py-0.5"
							>
								<Link
									to="/agents/$agentId/{-$tab}"
									params={{ agentId: a.id, tab: 'connectors' }}
									className="hover:underline"
								>
									{shortName(a.name)}
								</Link>
								<button
									type="button"
									aria-label={`Retirer ${shortName(a.name)} de ${connector.name}`}
									className="text-muted-foreground hover:text-foreground"
									onClick={() => onRevoke(a.id, connector.name)}
								>
									<X aria-hidden="true" className="size-3" />
								</button>
							</span>
						))
					)}
				</div>
				{hasAccount && candidates.length > 0 ? (
					<div className="flex items-center gap-2">
						<NativeSelect
							aria-label={`Agent à autoriser pour ${connector.name}`}
							value={selected}
							onChange={(e) => setPicked(e.target.value)}
						>
							{candidates.map((a) => (
								<option key={a.id} value={a.id}>
									{a.name}
								</option>
							))}
						</NativeSelect>
						<Button
							variant="outline"
							size="sm"
							aria-label={`Autoriser sur ${connector.name}`}
							onClick={() => onGrant(selected, connector.name)}
						>
							Autoriser
						</Button>
					</div>
				) : null}
			</div>
		</article>
	)
}
