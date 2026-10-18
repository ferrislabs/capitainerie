import { useState } from 'react'
import type { Schemas } from '#/api/api.client'
import { InlineAlert } from '#/components/layout/inline-alert'
import { PageHeader } from '#/components/layout/page-header'
import { SectionCard } from '#/components/layout/section-card'
import { TableSkeleton } from '#/components/layout/table-skeleton'
import { useIsAdmin } from '#/domain/identity/hooks/use-is-admin'
import { ManifestForm } from '#/domain/integrations/feature/manifest-form'
import { useGithubIntegration } from '#/domain/integrations/hooks/use-github-integration'
import {
	useCreateGithubManifest,
	useDeleteGithubApp,
	useDisconnectGithub,
	useStartGithubInstall,
} from '#/domain/integrations/hooks/use-github-mutations'
import { DeleteGithubAppDialog } from '#/domain/integrations/ui/delete-github-app-dialog'
import { DisconnectGithubDialog } from '#/domain/integrations/ui/disconnect-github-dialog'
import type { OwnerKind } from '#/domain/integrations/ui/github-app-form'
import { GithubPanel } from '#/domain/integrations/ui/github-panel'

interface Props {
	justConnected?: boolean
	appCreated?: boolean
}

export function IntegrationsPage({
	justConnected = false,
	appCreated = false,
}: Props) {
	const isAdmin = useIsAdmin()
	const github = useGithubIntegration()
	const install = useStartGithubInstall()
	const disconnect = useDisconnectGithub()
	const manifest = useCreateGithubManifest()
	const deleteApp = useDeleteGithubApp()
	const [confirming, setConfirming] = useState(false)
	const [disconnected, setDisconnected] = useState(false)
	const [confirmingDelete, setConfirmingDelete] = useState(false)
	const [deletedApp, setDeletedApp] = useState<Schemas.GithubAppResponse>()
	const [ownerKind, setOwnerKind] = useState<OwnerKind>('user')
	const [login, setLogin] = useState('')
	const [pending, setPending] = useState<Schemas.ManifestResponse>()

	function createApp() {
		manifest.create(
			ownerKind === 'user'
				? { type: 'user' }
				: { type: 'organization', login: login.trim() },
			{ onSuccess: setPending },
		)
	}

	return (
		<>
			<PageHeader
				title="Intégrations"
				description="Les services externes que les agents peuvent utiliser pour le compte de l’organisation."
			/>
			{appCreated && !deletedApp ? (
				<InlineAlert tone="success">
					L’application GitHub est créée. Installez-la sur vos dépôts.
				</InlineAlert>
			) : null}
			{justConnected && !disconnected && !deletedApp ? (
				<InlineAlert tone="success">GitHub est connecté.</InlineAlert>
			) : null}
			{disconnected ? (
				<InlineAlert tone="success">GitHub est déconnecté.</InlineAlert>
			) : null}
			{deletedApp ? (
				<InlineAlert tone="success">
					L’application est supprimée de Capitainerie. Supprimez-la aussi sur
					GitHub :{' '}
					<a
						href={deletedApp.html_url}
						target="_blank"
						rel="noreferrer"
						className="underline underline-offset-2"
					>
						ouvrir la page de l’application
					</a>
					.
				</InlineAlert>
			) : null}
			<SectionCard
				title="GitHub"
				description="Claude Code lit vos dépôts, pousse une branche et ouvre une pull request en brouillon."
			>
				{github.isError ? (
					<InlineAlert tone="danger">
						Impossible de charger l’état de la connexion GitHub.
					</InlineAlert>
				) : github.isPending ? (
					<TableSkeleton rows={2} columns={2} />
				) : (
					<GithubPanel
						status={github.data}
						isAdmin={isAdmin}
						create={{
							ownerKind,
							login,
							isPending: manifest.isPending || pending !== undefined,
							errorMessage: manifest.errorMessage,
							onOwnerKindChange: setOwnerKind,
							onLoginChange: setLogin,
							onSubmit: createApp,
						}}
						isConnecting={install.isPending}
						errorMessage={install.errorMessage}
						onConnect={install.start}
						onDisconnect={() => {
							disconnect.reset()
							setConfirming(true)
						}}
						onDeleteApp={() => {
							deleteApp.reset()
							setConfirmingDelete(true)
						}}
					/>
				)}
			</SectionCard>
			{pending ? (
				<ManifestForm
					actionUrl={pending.action_url}
					manifest={pending.manifest}
				/>
			) : null}
			<DisconnectGithubDialog
				open={confirming}
				isPending={disconnect.isPending}
				errorMessage={disconnect.errorMessage}
				onConfirm={() =>
					disconnect.remove({
						onSuccess: () => {
							setConfirming(false)
							setDisconnected(true)
						},
					})
				}
				onClose={() => setConfirming(false)}
			/>
			<DeleteGithubAppDialog
				open={confirmingDelete}
				htmlUrl={github.data?.app?.html_url}
				isPending={deleteApp.isPending}
				errorMessage={deleteApp.errorMessage}
				onConfirm={() =>
					deleteApp.remove({
						onSuccess: (deleted) => {
							setConfirmingDelete(false)
							setDeletedApp(deleted)
							setDisconnected(false)
						},
					})
				}
				onClose={() => setConfirmingDelete(false)}
			/>
		</>
	)
}
