import { Cpu, KeyRound } from 'lucide-react'
import { Fragment, type ReactNode } from 'react'
import type { Schemas } from '#/api/api.client'
import { EmptyState } from '#/components/layout/empty-state'
import { InlineAlert } from '#/components/layout/inline-alert'
import { StatusBadge } from '#/components/layout/status-badge'
import { Badge } from '#/components/ui/badge'
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from '#/components/ui/table'
import { formatContextWindow, formatPrice } from '#/domain/settings/format'
import {
	PROVIDER_INFO,
	PROVIDER_KINDS,
	providerLabel,
} from '#/domain/settings/providers'

interface Props {
	models: ReadonlyArray<Schemas.ModelResponse>
	errors: ReadonlyArray<Schemas.ProviderErrorResponse>
	hasProviders: boolean
	emptyAction?: ReactNode
}

const COLUMN_COUNT = 4

export function ModelsCatalog({
	models,
	errors,
	hasProviders,
	emptyAction,
}: Props) {
	if (!hasProviders) {
		return (
			<EmptyState
				icon={KeyRound}
				title="Aucun fournisseur configuré"
				description="Configurez un fournisseur pour voir les modèles qu’il annonce et les assigner aux rôles d’agent."
				action={emptyAction}
			/>
		)
	}

	const groups = PROVIDER_KINDS.map((provider) => ({
		provider,
		models: models.filter((model) => model.provider === provider),
		error: errors.find((error) => error.provider === provider),
	})).filter((group) => group.models.length > 0 || group.error)

	return (
		<div className="flex flex-col">
			{errors.length > 0 ? (
				<div className="flex flex-col gap-2 pb-3">
					{errors.map((error) => (
						<InlineAlert key={error.provider} tone="danger">
							{providerLabel(error.provider)} injoignable : {error.message}
						</InlineAlert>
					))}
				</div>
			) : null}
			{groups.length === 0 ? (
				<EmptyState
					icon={Cpu}
					title="Aucun modèle annoncé"
					description="Les fournisseurs configurés n’annoncent aucun modèle pour l’instant."
				/>
			) : (
				<Table>
					<TableHeader>
						<TableRow>
							<TableHead>Modèle</TableHead>
							<TableHead>Capacités</TableHead>
							<TableHead className="text-right">Contexte</TableHead>
							<TableHead className="text-right">
								Prix entrée / sortie
								<span className="font-normal"> (1M tokens)</span>
							</TableHead>
						</TableRow>
					</TableHeader>
					<TableBody>
						{groups.map((group) => (
							<Fragment key={group.provider}>
								<TableRow className="[tbody_&]:hover:bg-transparent">
									<TableCell colSpan={COLUMN_COUNT} className="h-8 bg-muted/50">
										<div className="flex items-center gap-2">
											<Badge variant="secondary">
												{PROVIDER_INFO[group.provider].label}
											</Badge>
											{group.error ? (
												<StatusBadge tone="danger" label="Injoignable" />
											) : (
												<span className="text-xs text-muted-foreground figures">
													{group.models.length} modèle
													{group.models.length > 1 ? 's' : ''}
												</span>
											)}
										</div>
									</TableCell>
								</TableRow>
								{group.models.map((model) => (
									<TableRow key={`${model.provider}:${model.id}`}>
										<TableCell>
											<div className="flex min-w-0 flex-col">
												<span className="font-mono text-xs font-medium break-all">
													{model.id}
												</span>
												{model.display_name &&
												model.display_name !== model.id ? (
													<span className="text-xs text-muted-foreground">
														{model.display_name}
													</span>
												) : null}
											</div>
										</TableCell>
										<TableCell>
											<div className="flex flex-wrap gap-1">
												{model.capabilities.chat ? (
													<Badge variant="outline">Chat</Badge>
												) : null}
												{model.capabilities.embeddings ? (
													<Badge variant="outline">Embeddings</Badge>
												) : null}
												{model.capabilities.transcription ? (
													<Badge variant="outline">Transcription</Badge>
												) : null}
											</div>
										</TableCell>
										<TableCell className="text-right whitespace-nowrap figures">
											{formatContextWindow(model.context_window)}
										</TableCell>
										<TableCell className="text-right whitespace-nowrap figures">
											{formatPrice(model.price)}
										</TableCell>
									</TableRow>
								))}
							</Fragment>
						))}
					</TableBody>
				</Table>
			)}
		</div>
	)
}
