import { Circle, CircleCheck, Cpu, Github, KeyRound } from 'lucide-react'
import type { ReactNode } from 'react'
import { LinkCard } from '#/components/layout/link-card'
import { PageHeader } from '#/components/layout/page-header'
import { Skeleton } from '#/components/ui/skeleton'
import { useGithubIntegration } from '#/domain/integrations/hooks/use-github-integration'
import {
	useModelAssignments,
	useProviders,
} from '#/domain/settings/hooks/use-settings-queries'
import {
	integrationsState,
	modelsState,
	providersState,
	type SettingState,
} from '#/domain/settings/overview'

interface Query {
	isError: boolean
	isPending: boolean
}

function State({
	query,
	state,
}: {
	query: Query
	state: () => SettingState
}): ReactNode {
	if (query.isError) {
		return <span className="text-base font-medium">Indisponible</span>
	}
	if (query.isPending) {
		return (
			<>
				<Skeleton className="h-5 w-32" />
				<span className="sr-only">Chargement</span>
			</>
		)
	}
	const { label, ready } = state()
	const Icon = ready ? CircleCheck : Circle
	return (
		<span className="flex items-center gap-2 text-base font-medium">
			<Icon
				aria-hidden="true"
				className={
					ready ? 'size-4 text-signal-success' : 'size-4 text-muted-foreground'
				}
			/>
			{label}
		</span>
	)
}

export function SettingsIndexPage() {
	const providers = useProviders()
	const assignments = useModelAssignments()
	const github = useGithubIntegration()

	return (
		<>
			<PageHeader
				title="Paramètres"
				description="Les fournisseurs, les modèles par rôle et les services connectés."
			/>
			<div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
				<LinkCard
					to="/settings/providers"
					icon={KeyRound}
					title="Fournisseurs"
					value={
						<State
							query={providers}
							state={() => providersState(providers.data?.providers ?? [])}
						/>
					}
					hint="Les clés d’API des fournisseurs de modèles."
				/>
				<LinkCard
					to="/settings/models"
					icon={Cpu}
					title="Modèles"
					value={
						<State
							query={assignments}
							state={() => modelsState(assignments.data?.assignments)}
						/>
					}
					hint="Le modèle utilisé par chaque agent."
				/>
				<LinkCard
					to="/settings/integrations"
					icon={Github}
					title="Intégrations"
					value={
						<State
							query={github}
							state={() => integrationsState(github.data ?? {})}
						/>
					}
					hint="Les services externes que les agents utilisent."
				/>
			</div>
		</>
	)
}
