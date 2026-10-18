import { Link } from '@tanstack/react-router'
import { useState } from 'react'
import { InlineAlert } from '#/components/layout/inline-alert'
import { PageHeader } from '#/components/layout/page-header'
import { SectionCard } from '#/components/layout/section-card'
import { TableSkeleton } from '#/components/layout/table-skeleton'
import { Button } from '#/components/ui/button'
import { useIsAdmin } from '#/domain/identity/hooks/use-is-admin'
import { useSaveAssignments } from '#/domain/settings/hooks/use-save-assignments'
import {
	useModelAssignments,
	useModelCatalog,
	useProviders,
} from '#/domain/settings/hooks/use-settings-queries'
import { ADMIN_ONLY_MODELS } from '#/domain/settings/providers'
import {
	AGENT_ROLES,
	type AssignmentDraft,
	completeAssignments,
	sameModelRef,
} from '#/domain/settings/roles'
import { ModelAssignmentsForm } from '#/domain/settings/ui/model-assignments-form'
import { ModelsCatalog } from '#/domain/settings/ui/models-catalog'

export function ModelsPage() {
	const isAdmin = useIsAdmin()
	const providers = useProviders()
	const catalog = useModelCatalog()
	const assignments = useModelAssignments()
	const saveAssignments = useSaveAssignments()
	const [edits, setEdits] = useState<AssignmentDraft>({})

	const saved = assignments.data?.assignments ?? null
	const values: AssignmentDraft = { ...(saved ?? {}), ...edits }
	const complete = completeAssignments(values)
	const isDirty = AGENT_ROLES.some(
		(role) => !sameModelRef(values[role], saved?.[role]),
	)
	const embeddingsChanged =
		saved !== null &&
		values.embeddings !== undefined &&
		!sameModelRef(values.embeddings, saved.embeddings)

	const models = catalog.data?.models ?? []
	const hasProviders = (providers.data?.providers.length ?? 0) > 0

	function submit() {
		if (!complete) return
		saveAssignments.save(complete, { onSuccess: () => setEdits({}) })
	}

	return (
		<>
			<PageHeader
				title="Modèles"
				description="Les modèles annoncés par vos fournisseurs, et celui qui sert chaque rôle d'agent."
			/>
			<SectionCard
				title="Modèle par rôle"
				description="Les rôles de chat proposent les modèles de chat ; Embeddings, les modèles d’embeddings."
				flush
			>
				{assignments.isError ? (
					<InlineAlert tone="danger" className="m-4">
						Impossible de charger les modèles par rôle.
					</InlineAlert>
				) : assignments.isPending || catalog.isPending ? (
					<TableSkeleton rows={4} columns={2} />
				) : (
					<ModelAssignmentsForm
						models={models}
						values={values}
						usesWorkerDefault={saved === null}
						isAdmin={isAdmin}
						readOnlyMessage={ADMIN_ONLY_MODELS}
						canSubmit={complete !== undefined && isDirty}
						isPending={saveAssignments.isPending}
						embeddingsChanged={embeddingsChanged}
						errorMessage={saveAssignments.errorMessage}
						successMessage={
							saveAssignments.isSuccess && !isDirty
								? 'Modèles enregistrés. Ils serviront aux prochains runs.'
								: undefined
						}
						onChange={(role, ref) => {
							if (saveAssignments.isSuccess || saveAssignments.errorMessage) {
								saveAssignments.reset()
							}
							setEdits((previous) => ({ ...previous, [role]: ref }))
						}}
						onSubmit={submit}
					/>
				)}
			</SectionCard>
			<SectionCard
				title="Catalogue"
				description="Les modèles annoncés par chaque fournisseur configuré. Prix en dollars par million de tokens, quand ils sont connus."
				flush
			>
				{catalog.isError || providers.isError ? (
					<InlineAlert tone="danger" className="m-4">
						Impossible de charger le catalogue des modèles.
					</InlineAlert>
				) : catalog.isPending || providers.isPending ? (
					<TableSkeleton rows={4} columns={4} />
				) : (
					<ModelsCatalog
						models={models}
						errors={catalog.data.errors}
						hasProviders={hasProviders}
						emptyAction={
							<Button asChild variant="outline" size="sm">
								<Link to="/settings/providers">Configurer un fournisseur</Link>
							</Button>
						}
					/>
				)}
			</SectionCard>
		</>
	)
}
