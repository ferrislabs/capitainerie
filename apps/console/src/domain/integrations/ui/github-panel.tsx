import { Github } from 'lucide-react'
import type { Schemas } from '#/api/api.client'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Button } from '#/components/ui/button'
import {
	GithubAppForm,
	type OwnerKind,
} from '#/domain/integrations/ui/github-app-form'

const dateFormat = new Intl.DateTimeFormat('fr-FR', { dateStyle: 'long' })

export interface CreateAppProps {
	ownerKind: OwnerKind
	login: string
	isPending: boolean
	errorMessage?: string
	onOwnerKindChange: (kind: OwnerKind) => void
	onLoginChange: (login: string) => void
	onSubmit: () => void
}

interface Props {
	status: Schemas.GithubIntegrationResponse
	isAdmin: boolean
	create: CreateAppProps
	isConnecting: boolean
	errorMessage?: string
	onConnect: () => void
	onDisconnect: () => void
	onDeleteApp: () => void
}

function formatDate(iso: string | null | undefined): string | undefined {
	if (!iso) return undefined
	const date = new Date(iso)
	return Number.isNaN(date.getTime()) ? undefined : dateFormat.format(date)
}

function AppLine({ app }: { app: Schemas.GithubAppResponse }) {
	const createdAt = formatDate(app.created_at)
	return (
		<p className="text-[13px] text-muted-foreground">
			Application GitHub{' '}
			<a
				href={app.html_url}
				target="_blank"
				rel="noreferrer"
				className="font-mono text-foreground underline underline-offset-2"
			>
				{app.slug}
			</a>
			, propriétaire <span className="font-mono">{app.owner_login}</span>
			{createdAt ? `, créée le ${createdAt}` : ''}.
		</p>
	)
}

export function GithubPanel({
	status,
	isAdmin,
	create,
	isConnecting,
	errorMessage,
	onConnect,
	onDisconnect,
	onDeleteApp,
}: Props) {
	const app = status.app
	const installation = status.installation

	if (!app) {
		if (!isAdmin) {
			return (
				<p className="text-[13px] text-muted-foreground">
					L’organisation n’a pas encore d’application GitHub. Seuls les
					administrateurs de l’organisation peuvent la créer.
				</p>
			)
		}
		return <GithubAppForm {...create} />
	}

	if (!installation) {
		return (
			<div className="flex flex-col items-start gap-3">
				<AppLine app={app} />
				<p className="text-sm text-muted-foreground">
					Installez l’application sur les dépôts que Claude Code peut modifier.
				</p>
				{isAdmin ? (
					<div className="flex flex-wrap gap-2">
						<Button type="button" disabled={isConnecting} onClick={onConnect}>
							<Github aria-hidden="true" />
							{isConnecting ? 'Redirection…' : 'Installer sur vos dépôts'}
						</Button>
						<Button type="button" variant="outline" onClick={onDeleteApp}>
							Supprimer l’application
						</Button>
					</div>
				) : (
					<p className="text-[13px] text-muted-foreground">
						GitHub n’est pas connecté. Seuls les administrateurs de
						l’organisation peuvent installer l’application.
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

	const connectedAt = formatDate(installation.connected_at)

	return (
		<div className="flex flex-col gap-4">
			<dl className="grid gap-x-6 gap-y-3 sm:grid-cols-3">
				<div className="flex flex-col gap-0.5">
					<dt className="text-xs text-muted-foreground">Compte GitHub</dt>
					<dd className="font-mono text-sm">{installation.account_login}</dd>
				</div>
				<div className="flex flex-col gap-0.5">
					<dt className="text-xs text-muted-foreground">Connecté par</dt>
					<dd className="text-sm">{installation.connected_by}</dd>
				</div>
				<div className="flex flex-col gap-0.5">
					<dt className="text-xs text-muted-foreground">Connecté le</dt>
					<dd className="text-sm figures">{connectedAt ?? '—'}</dd>
				</div>
			</dl>
			<AppLine app={app} />
			{errorMessage ? (
				<InlineAlert tone="danger">{errorMessage}</InlineAlert>
			) : null}
			{isAdmin ? (
				<div className="flex flex-wrap gap-2">
					<Button type="button" variant="outline" onClick={onDisconnect}>
						Déconnecter
					</Button>
					<Button type="button" variant="outline" onClick={onDeleteApp}>
						Supprimer l’application
					</Button>
				</div>
			) : null}
		</div>
	)
}
