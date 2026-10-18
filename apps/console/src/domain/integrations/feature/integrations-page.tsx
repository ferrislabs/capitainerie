import { useState } from 'react'
import { InlineAlert } from '#/components/layout/inline-alert'
import { PageHeader } from '#/components/layout/page-header'
import { SectionCard } from '#/components/layout/section-card'
import { TableSkeleton } from '#/components/layout/table-skeleton'
import { useIsAdmin } from '#/domain/identity/hooks/use-is-admin'
import { useGithubIntegration } from '#/domain/integrations/hooks/use-github-integration'
import {
	useDisconnectGithub,
	useStartGithubInstall,
} from '#/domain/integrations/hooks/use-github-mutations'
import { DisconnectGithubDialog } from '#/domain/integrations/ui/disconnect-github-dialog'
import { GithubPanel } from '#/domain/integrations/ui/github-panel'

interface Props {
	justConnected?: boolean
}

export function IntegrationsPage({ justConnected = false }: Props) {
	const isAdmin = useIsAdmin()
	const github = useGithubIntegration()
	const install = useStartGithubInstall()
	const disconnect = useDisconnectGithub()
	const [confirming, setConfirming] = useState(false)
	const [disconnected, setDisconnected] = useState(false)

	return (
		<>
			<PageHeader
				title="Intégrations"
				description="Les services externes que les agents peuvent utiliser pour le compte de l’organisation."
			/>
			{justConnected && !disconnected ? (
				<InlineAlert tone="success">GitHub est connecté.</InlineAlert>
			) : null}
			{disconnected ? (
				<InlineAlert tone="success">GitHub est déconnecté.</InlineAlert>
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
						isConnecting={install.isPending}
						errorMessage={install.errorMessage}
						onConnect={install.start}
						onDisconnect={() => {
							disconnect.reset()
							setConfirming(true)
						}}
					/>
				)}
			</SectionCard>
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
		</>
	)
}
