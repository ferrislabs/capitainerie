import { Github } from 'lucide-react'
import type { Schemas } from '#/api/api.client'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Button } from '#/components/ui/button'

const dateFormat = new Intl.DateTimeFormat('fr-FR', { dateStyle: 'long' })

interface Props {
	status: Schemas.GithubIntegrationResponse
	isAdmin: boolean
	isConnecting: boolean
	errorMessage?: string
	onConnect: () => void
	onDisconnect: () => void
}

function formatDate(iso: string | null | undefined): string | undefined {
	if (!iso) return undefined
	const date = new Date(iso)
	return Number.isNaN(date.getTime()) ? undefined : dateFormat.format(date)
}

export function GithubPanel({
	status,
	isAdmin,
	isConnecting,
	errorMessage,
	onConnect,
	onDisconnect,
}: Props) {
	if (!status.app_configured) {
		return (
			<div className="flex flex-col gap-2">
				<p className="text-sm font-medium">
					L’application GitHub n’est pas configurée sur ce serveur.
				</p>
				<p className="text-[13px] text-muted-foreground">
					L’exploitant doit renseigner les variables{' '}
					<span className="font-mono text-xs">GITHUB_APP_*</span> de l’API, puis
					la redémarrer. La procédure est décrite dans le README, section «
					Exécutant ».
				</p>
			</div>
		)
	}

	if (!status.connected) {
		return (
			<div className="flex flex-col items-start gap-3">
				<p className="text-sm text-muted-foreground">
					Connectez le compte GitHub qui héberge vos dépôts pour confier des
					cartes à Claude Code.
				</p>
				{isAdmin ? (
					<Button type="button" disabled={isConnecting} onClick={onConnect}>
						<Github aria-hidden="true" />
						{isConnecting ? 'Redirection…' : 'Connecter GitHub'}
					</Button>
				) : (
					<p className="text-[13px] text-muted-foreground">
						GitHub n’est pas connecté. Seuls les administrateurs de
						l’organisation peuvent le connecter.
					</p>
				)}
				{errorMessage ? (
					<InlineAlert tone="danger" className="w-full">
						{errorMessage}
					</InlineAlert>
				) : null}
			</div>
		)
	}

	const connectedAt = formatDate(status.connected_at)

	return (
		<div className="flex flex-col gap-4">
			<dl className="grid gap-x-6 gap-y-3 sm:grid-cols-3">
				<div className="flex flex-col gap-0.5">
					<dt className="text-xs text-muted-foreground">Compte GitHub</dt>
					<dd className="font-mono text-sm">{status.account_login ?? '—'}</dd>
				</div>
				<div className="flex flex-col gap-0.5">
					<dt className="text-xs text-muted-foreground">Connecté par</dt>
					<dd className="text-sm">{status.connected_by ?? '—'}</dd>
				</div>
				<div className="flex flex-col gap-0.5">
					<dt className="text-xs text-muted-foreground">Connecté le</dt>
					<dd className="text-sm figures">{connectedAt ?? '—'}</dd>
				</div>
			</dl>
			{errorMessage ? (
				<InlineAlert tone="danger">{errorMessage}</InlineAlert>
			) : null}
			{isAdmin ? (
				<div>
					<Button type="button" variant="outline" onClick={onDisconnect}>
						Déconnecter
					</Button>
				</div>
			) : null}
		</div>
	)
}
