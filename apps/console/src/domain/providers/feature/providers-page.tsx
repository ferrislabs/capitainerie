import { useState } from 'react'
import type { Schemas } from '#/api/api.client'
import { useNotice } from '#/components/layout/notice'
import {
	AlertDialog,
	AlertDialogAction,
	AlertDialogCancel,
	AlertDialogContent,
	AlertDialogDescription,
	AlertDialogFooter,
	AlertDialogHeader,
	AlertDialogTitle,
} from '#/components/ui/alert-dialog'
import {
	useDeleteMyProvider,
	useMyProviders,
	usePutMyProvider,
	useTestMyProvider,
} from '#/domain/providers/hooks/use-providers'
import {
	buildPutBody,
	describeDeleteError,
	describeSaveError,
	describeTestError,
	type ProviderDraft,
	type ProviderKind,
	type ProviderSpec,
} from '#/domain/providers/model'
import type { TestState } from '#/domain/providers/ui/provider-card'
import { ProviderDialog } from '#/domain/providers/ui/provider-dialog'
import { ProvidersView } from '#/domain/providers/ui/providers-view'

export function ProvidersPage() {
	const query = useMyProviders()
	const put = usePutMyProvider()
	const remove = useDeleteMyProvider()
	const test = useTestMyProvider()
	const [notice, setNotice] = useNotice()
	const [editing, setEditing] = useState<ProviderSpec | null>(null)
	const [deleting, setDeleting] = useState<ProviderSpec | null>(null)
	const [saveError, setSaveError] = useState<string | null>(null)
	const [tests, setTests] = useState<Partial<Record<ProviderKind, TestState>>>(
		{},
	)

	const providers = query.data ?? []
	const busyKinds = [
		put.isPending ? put.variables?.path.kind : undefined,
		remove.isPending ? remove.variables?.path.kind : undefined,
		test.isPending ? test.variables?.path.kind : undefined,
	].filter((kind): kind is ProviderKind => kind !== undefined)

	const closeDialog = () => {
		setEditing(null)
		setSaveError(null)
	}

	const save = (spec: ProviderSpec, draft: ProviderDraft) => {
		setSaveError(null)
		put.mutate(
			{ path: { kind: spec.kind }, body: buildPutBody(spec, draft) },
			{
				onSuccess: () => {
					closeDialog()
					setTests(({ [spec.kind]: _removed, ...rest }) => rest)
					setNotice(`${spec.name} enregistré`)
				},
				onError: (error) => setSaveError(describeSaveError(error)),
			},
		)
	}

	const makeDefault = (provider: Schemas.UserProviderResponse) =>
		put.mutate(
			{
				path: { kind: provider.kind },
				body: {
					default_model: provider.default_model,
					is_default: true,
					...(provider.base_url ? { base_url: provider.base_url } : {}),
				},
			},
			{
				onSuccess: () => setNotice('Fournisseur défini par défaut'),
				onError: (error) => setNotice(describeSaveError(error)),
			},
		)

	const runTest = (spec: ProviderSpec) =>
		test.mutate(
			{ path: { kind: spec.kind } },
			{
				onSuccess: (result) =>
					setTests((prev) => ({
						...prev,
						[spec.kind]: result.ok
							? { ok: true }
							: {
									ok: false,
									message:
										result.error ?? 'Le fournisseur a refusé la connexion.',
								},
					})),
				onError: (error) =>
					setTests((prev) => ({
						...prev,
						[spec.kind]: { ok: false, message: describeTestError(error) },
					})),
			},
		)

	const confirmDelete = () => {
		if (!deleting) return
		const spec = deleting
		setDeleting(null)
		remove.mutate(
			{ path: { kind: spec.kind } },
			{
				onSuccess: () => {
					setTests(({ [spec.kind]: _removed, ...rest }) => rest)
					setNotice(`${spec.name} supprimé`)
				},
				onError: (error) => setNotice(describeDeleteError(error)),
			},
		)
	}

	return (
		<>
			<ProvidersView
				providers={providers}
				loading={query.isPending}
				error={query.isError}
				notice={notice}
				busyKinds={busyKinds}
				tests={tests}
				onRetry={() => void query.refetch()}
				onEdit={setEditing}
				onTest={runTest}
				onDelete={setDeleting}
				onMakeDefault={makeDefault}
			/>
			<ProviderDialog
				spec={editing}
				existing={providers.find((p) => p.kind === editing?.kind)}
				firstProvider={providers.length === 0}
				pending={put.isPending}
				error={saveError}
				onSubmit={save}
				onClose={closeDialog}
			/>
			<AlertDialog
				open={deleting !== null}
				onOpenChange={(next) => (next ? null : setDeleting(null))}
			>
				<AlertDialogContent>
					<AlertDialogHeader>
						<AlertDialogTitle>Supprimer {deleting?.name} ?</AlertDialogTitle>
						<AlertDialogDescription>
							Votre clé sera effacée. Les agents que vous lancez ne pourront
							plus s'exécuter avec ce fournisseur.
						</AlertDialogDescription>
					</AlertDialogHeader>
					<AlertDialogFooter>
						<AlertDialogCancel>Annuler</AlertDialogCancel>
						<AlertDialogAction onClick={confirmDelete}>
							Supprimer
						</AlertDialogAction>
					</AlertDialogFooter>
				</AlertDialogContent>
			</AlertDialog>
		</>
	)
}
