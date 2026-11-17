import { Link } from '@tanstack/react-router'
import { Plus } from 'lucide-react'
import { useState } from 'react'
import { Button } from '#/components/ui/button'
import { NativeSelect } from '#/components/ui/native-select'
import { HumanAvatar } from '#/domain/audit/ui/avatar'
import { Sev } from '#/domain/audit/ui/sev'
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
		<article className="flex min-w-0 flex-col gap-2 overflow-hidden rounded-lg border bg-card px-3 py-[11px]">
			<div className="flex items-start gap-2.5">
				<HumanAvatar initials={connector.name.charAt(0)} size={28} />
				<div className="min-w-0 flex-1">
					<h3 className="m-0 text-[13px] font-semibold">
						{connector.name}{' '}
						<span className="font-mono text-[11px] font-medium whitespace-nowrap text-faint">
							{connector.kind}
						</span>
					</h3>
					<p className="m-0 text-muted-foreground">{connector.description}</p>
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
					<div
						key={account}
						className="flex items-center gap-2 border-t pt-[7px]"
					>
						<Sev tone="ok">Connecté</Sev>
						<span className="min-w-0 flex-1 truncate">{account}</span>
					</div>
				))
			) : (
				<div className="border-t pt-[7px] text-faint">
					Aucun compte connecté
				</div>
			)}
			<div className="border-t pt-[7px]">
				<div className="mb-[5px] text-xs text-muted-foreground">
					{toolCount} outil{toolCount > 1 ? 's' : ''} · agents autorisés
				</div>
				<div className="flex flex-wrap gap-1.5">
					{granted.length === 0 ? (
						<span className="text-faint">
							Aucun : accès refusé à tous les agents
						</span>
					) : (
						granted.map((a) => (
							<span
								key={a.id}
								className="inline-flex items-center gap-1 rounded-full border bg-background py-0.5 pr-1 pl-[9px] font-mono text-xs"
							>
								<Link
									to="/agents/$agentId/{-$tab}"
									params={{ agentId: a.id, tab: 'connectors' }}
								>
									{shortName(a.name)}
								</Link>
								<button
									type="button"
									aria-label={`Retirer ${shortName(a.name)} de ${connector.name}`}
									className="size-[18px] rounded-full p-0 leading-none text-faint hover:bg-secondary hover:text-signal-danger"
									onClick={() => onRevoke(a.id, connector.name)}
								>
									×
								</button>
							</span>
						))
					)}
				</div>
				{hasAccount && candidates.length > 0 ? (
					<div className="flex gap-1.5 pt-2">
						<NativeSelect
							aria-label={`Agent à autoriser pour ${connector.name}`}
							className="min-w-0 flex-1"
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
