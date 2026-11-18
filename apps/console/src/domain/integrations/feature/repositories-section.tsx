import { InlineAlert } from '#/components/layout/inline-alert'
import { SectionCard } from '#/components/layout/section-card'
import { TableSkeleton } from '#/components/layout/table-skeleton'
import { Button } from '#/components/ui/button'
import {
	useGithubIntegration,
	useGithubRepositories,
} from '#/domain/integrations/hooks/use-github'
import { RepositoriesList } from '#/domain/integrations/ui/repositories-list'
import { hasErrorKey } from '#/lib/api-error'

const TITLE = 'Dépôts GitHub accessibles'

export function RepositoriesSection() {
	const integration = useGithubIntegration()
	const installation = integration.data?.installation ?? null
	const notConnected = integration.data !== undefined && installation === null
	const repositories = useGithubRepositories(installation !== null)

	if (integration.isError) {
		return (
			<SectionCard title={TITLE}>
				<InlineAlert tone="danger">
					Impossible de lire l'état de l'intégration GitHub.
				</InlineAlert>
			</SectionCard>
		)
	}

	if (notConnected || hasErrorKey(repositories.error, 'github_not_connected')) {
		return (
			<SectionCard title={TITLE}>
				<InlineAlert tone="info">
					GitHub n'est pas connecté à l'organisation : les chantiers ne peuvent
					pas encore travailler sur un dépôt.
				</InlineAlert>
			</SectionCard>
		)
	}

	return (
		<SectionCard
			title={TITLE}
			description={
				installation
					? `Installation sur ${installation.account_login} · modifie-la sur GitHub pour ajouter ou retirer un dépôt`
					: undefined
			}
			flush
		>
			{repositories.isError ? (
				<div className="flex flex-col items-start gap-2 p-3">
					<InlineAlert tone="danger" className="w-full">
						Impossible de charger les dépôts GitHub.
					</InlineAlert>
					<Button
						type="button"
						size="sm"
						variant="outline"
						onClick={() => void repositories.refetch()}
					>
						Réessayer
					</Button>
				</div>
			) : repositories.data ? (
				<RepositoriesList repositories={repositories.data.repositories} />
			) : (
				<div className="p-3">
					<TableSkeleton rows={3} columns={2} />
				</div>
			)}
		</SectionCard>
	)
}
