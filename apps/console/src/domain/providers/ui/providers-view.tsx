import { KeyRound } from 'lucide-react'
import type { Schemas } from '#/api/api.client'
import { EmptyState } from '#/components/layout/empty-state'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Notice } from '#/components/layout/notice'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { Button } from '#/components/ui/button'
import {
	PROVIDER_SPECS,
	type ProviderKind,
	type ProviderSpec,
} from '#/domain/providers/model'
import {
	ProviderCard,
	type TestState,
} from '#/domain/providers/ui/provider-card'

interface Props {
	providers: ReadonlyArray<Schemas.UserProviderResponse>
	loading: boolean
	error: boolean
	notice: string | null
	busyKinds: ReadonlyArray<ProviderKind>
	tests: Readonly<Partial<Record<ProviderKind, TestState>>>
	onRetry: () => void
	onEdit: (spec: ProviderSpec) => void
	onTest: (spec: ProviderSpec) => void
	onDelete: (spec: ProviderSpec) => void
	onMakeDefault: (provider: Schemas.UserProviderResponse) => void
}

export function ProvidersView({
	providers,
	loading,
	error,
	notice,
	busyKinds,
	tests,
	onRetry,
	onEdit,
	onTest,
	onDelete,
	onMakeDefault,
}: Props) {
	return (
		<Page>
			<PageHeader
				title="Fournisseurs LLM"
				description="Vos propres clés : les agents que vous lancez s'exécutent avec votre fournisseur"
			/>
			<Notice message={notice} className="mb-3" />
			{error ? (
				<div className="grid gap-2">
					<InlineAlert tone="danger">
						Impossible de charger vos fournisseurs.
					</InlineAlert>
					<div>
						<Button variant="outline" onClick={onRetry}>
							Réessayer
						</Button>
					</div>
				</div>
			) : loading ? (
				<output className="m-0 text-muted-foreground">
					Chargement des fournisseurs…
				</output>
			) : (
				<div className="grid gap-3">
					{providers.length === 0 ? (
						<EmptyState
							icon={KeyRound}
							title="Aucun fournisseur connecté"
							description="Les agents que vous lancez s'exécutent avec votre propre fournisseur LLM. Connectez-en un ci-dessous pour pouvoir les exécuter."
						/>
					) : null}
					<div className="grid grid-cols-1 gap-2.5 md:grid-cols-2 xl:grid-cols-3">
						{PROVIDER_SPECS.map((spec) => (
							<ProviderCard
								key={spec.kind}
								spec={spec}
								provider={providers.find((p) => p.kind === spec.kind)}
								busy={busyKinds.includes(spec.kind)}
								test={tests[spec.kind]}
								onEdit={onEdit}
								onTest={onTest}
								onDelete={onDelete}
								onMakeDefault={onMakeDefault}
							/>
						))}
					</div>
				</div>
			)}
		</Page>
	)
}
