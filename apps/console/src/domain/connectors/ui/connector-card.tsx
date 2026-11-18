import { Archive, ArchiveRestore, Link2, Link2Off, Wrench } from 'lucide-react'
import type { Schemas } from '#/api/api.client'
import { Button } from '#/components/ui/button'
import { HumanAvatar } from '#/domain/audit/ui/avatar'
import { Sev } from '#/domain/audit/ui/sev'
import { expiryLabel, hostOf } from '#/domain/connectors/model'

export type ToolsState =
	| { kind: 'ok'; tools: ReadonlyArray<Schemas.McpToolResponse> }
	| { kind: 'error'; message: string }

interface Props {
	connector: Schemas.McpConnectorResponse
	busy: boolean
	tools: ToolsState | undefined
	onConnect: (id: string) => void
	onDisconnect: (id: string) => void
	onTest: (id: string) => void
	onArchive: (id: string, archived: boolean) => void
}

export function ConnectorCard({
	connector,
	busy,
	tools,
	onConnect,
	onDisconnect,
	onTest,
	onArchive,
}: Props) {
	const expiry = connector.connected ? expiryLabel(connector.expires_at) : null
	return (
		<article className="flex min-w-0 flex-col gap-2 overflow-hidden rounded-lg border bg-card px-3 py-[11px]">
			<div className="flex items-start gap-2.5">
				<HumanAvatar initials={connector.name.charAt(0)} size={28} />
				<div className="min-w-0 flex-1">
					<h3 className="m-0 text-[13px] font-semibold">
						{connector.name}{' '}
						<span className="font-mono text-[11px] font-medium whitespace-nowrap text-faint">
							MCP
						</span>
					</h3>
					<p className="m-0 truncate font-mono text-xs text-muted-foreground">
						{hostOf(connector.url)}
					</p>
				</div>
				<Button
					variant="outline"
					size="sm"
					disabled={busy}
					onClick={() => onArchive(connector.id, !connector.archived)}
				>
					{connector.archived ? (
						<ArchiveRestore aria-hidden="true" />
					) : (
						<Archive aria-hidden="true" />
					)}
					{connector.archived ? 'Restaurer' : 'Archiver'}
				</Button>
			</div>
			<div className="flex flex-wrap items-center gap-2 border-t pt-[7px]">
				<Sev tone={connector.connected ? 'ok' : 'info'}>
					{connector.connected ? 'Connecté' : 'Non connecté'}
				</Sev>
				{expiry ? (
					<span
						className={
							expiry.expired ? 'text-signal-danger' : 'text-muted-foreground'
						}
					>
						{expiry.text}
					</span>
				) : null}
				<div className="ml-auto flex flex-wrap gap-1.5">
					{connector.connected ? (
						<>
							<Button
								variant="outline"
								size="sm"
								disabled={busy}
								onClick={() => onTest(connector.id)}
							>
								<Wrench aria-hidden="true" />
								Tester
							</Button>
							<Button
								variant="outline"
								size="sm"
								disabled={busy}
								onClick={() => onDisconnect(connector.id)}
							>
								<Link2Off aria-hidden="true" />
								Se déconnecter
							</Button>
						</>
					) : (
						<Button
							size="sm"
							disabled={busy || connector.archived}
							onClick={() => onConnect(connector.id)}
						>
							<Link2 aria-hidden="true" />
							Se connecter
						</Button>
					)}
				</div>
			</div>
			{tools ? <ToolsResult state={tools} name={connector.name} /> : null}
		</article>
	)
}

function ToolsResult({ state, name }: { state: ToolsState; name: string }) {
	if (state.kind === 'error')
		return (
			<p role="alert" className="m-0 border-t pt-[7px] text-signal-danger">
				{state.message}
			</p>
		)
	return (
		<div className="border-t pt-[7px]">
			<div className="mb-[5px] text-xs text-muted-foreground">
				{state.tools.length} outil{state.tools.length > 1 ? 's' : ''} exposé
				{state.tools.length > 1 ? 's' : ''} par {name}
			</div>
			{state.tools.length === 0 ? (
				<span className="text-faint">Aucun outil</span>
			) : (
				<ul className="m-0 flex list-none flex-col gap-1 p-0">
					{state.tools.map((tool) => (
						<li key={tool.name} className="flex items-center gap-2">
							<span className="min-w-0 flex-1 truncate font-mono text-[11.5px]">
								{tool.name}
							</span>
							<Sev tone={tool.read_only ? 'ok' : 'warn'}>
								{tool.read_only ? 'lecture' : 'écriture'}
							</Sev>
						</li>
					))}
				</ul>
			)}
		</div>
	)
}
