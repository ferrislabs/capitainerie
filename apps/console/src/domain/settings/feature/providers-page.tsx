import { useForm, useStore } from '@tanstack/react-form'
import { useState } from 'react'
import { InlineAlert } from '#/components/layout/inline-alert'
import { PageHeader } from '#/components/layout/page-header'
import { SectionCard } from '#/components/layout/section-card'
import { TableSkeleton } from '#/components/layout/table-skeleton'
import { useIsAdmin } from '#/domain/identity/hooks/use-is-admin'
import { useMe } from '#/domain/identity/hooks/use-me'
import {
	useDeleteProvider,
	useSaveProvider,
	useTestProvider,
} from '#/domain/settings/hooks/use-provider-mutations'
import { useProviders } from '#/domain/settings/hooks/use-settings-queries'
import {
	ADMIN_ONLY_PROVIDERS,
	PROVIDER_INFO,
	type ProviderKind,
	validateApiKey,
	validateBaseUrl,
} from '#/domain/settings/providers'
import { DeleteProviderDialog } from '#/domain/settings/ui/delete-provider-dialog'
import { ProviderKeyDialog } from '#/domain/settings/ui/provider-key-dialog'
import { ProvidersList } from '#/domain/settings/ui/providers-list'

const EMPTY_FORM = { apiKey: '', baseUrl: '' }

export function ProvidersPage() {
	const isAdmin = useIsAdmin()
	const me = useMe()
	const providers = useProviders()
	const saveProvider = useSaveProvider()
	const deleteProvider = useDeleteProvider()
	const testProvider = useTestProvider()
	const [editingKind, setEditingKind] = useState<ProviderKind>()
	const [editorOpen, setEditorOpen] = useState(false)
	const [deletingKind, setDeletingKind] = useState<ProviderKind>()
	const [deleteOpen, setDeleteOpen] = useState(false)
	const [notice, setNotice] = useState<string>()

	const form = useForm({
		defaultValues: EMPTY_FORM,
		validators: {
			onChange: ({ value }) => {
				if (!editingKind) return undefined
				const apiKey = validateApiKey(editingKind, value.apiKey)
				const baseUrl = validateBaseUrl(editingKind, value.baseUrl)
				return apiKey || baseUrl ? { fields: { apiKey, baseUrl } } : undefined
			},
		},
		onSubmit: ({ value, formApi }) => {
			const kind = editingKind
			if (!kind) return
			saveProvider.save(
				kind,
				{ apiKey: value.apiKey, baseUrl: value.baseUrl },
				{
					onSuccess: () => {
						testProvider.forget(kind)
						setNotice(`${PROVIDER_INFO[kind].label} enregistré.`)
						setEditorOpen(false)
						formApi.reset()
					},
				},
			)
			formApi.setFieldValue('apiKey', '')
		},
	})

	const values = useStore(form.store, (state) => state.values)
	const fieldMeta = useStore(form.store, (state) => state.fieldMeta)
	const attempts = useStore(form.store, (state) => state.submissionAttempts)
	const list = providers.data?.providers ?? []

	function fieldError(name: 'apiKey' | 'baseUrl') {
		if (attempts === 0) return undefined
		const first = fieldMeta[name]?.errors[0]
		return typeof first === 'string' ? first : undefined
	}

	function openEditor(kind: ProviderKind) {
		saveProvider.reset()
		setNotice(undefined)
		const current = list.find((provider) => provider.kind === kind)
		form.reset()
		form.setFieldValue('baseUrl', current?.base_url ?? '')
		setEditingKind(kind)
		setEditorOpen(true)
	}

	function closeEditor() {
		setEditorOpen(false)
		form.reset()
		saveProvider.reset()
	}

	function openDelete(kind: ProviderKind) {
		deleteProvider.reset()
		setNotice(undefined)
		setDeletingKind(kind)
		setDeleteOpen(true)
	}

	function confirmDelete() {
		const kind = deletingKind
		if (!kind) return
		deleteProvider.remove(kind, {
			onSuccess: () => {
				testProvider.forget(kind)
				setDeleteOpen(false)
				setNotice(`${PROVIDER_INFO[kind].label} supprimé.`)
			},
		})
	}

	return (
		<>
			<PageHeader
				title="Fournisseurs"
				description="Les clés d'API des fournisseurs de modèles, partagées par tous les runs de l'organisation."
			/>
			{isAdmin ? null : (
				<InlineAlert tone="info">
					{ADMIN_ONLY_PROVIDERS} Vous voyez quels fournisseurs sont configurés,
					jamais leurs clés.
				</InlineAlert>
			)}
			{notice ? <InlineAlert tone="success">{notice}</InlineAlert> : null}
			<SectionCard
				title="Fournisseurs de l’organisation"
				description="Une clé par fournisseur. Après l’enregistrement, seuls ses 4 derniers caractères restent visibles."
				flush
			>
				{providers.isError ? (
					<InlineAlert tone="danger" className="m-4">
						Impossible de charger les fournisseurs.
					</InlineAlert>
				) : providers.isPending ? (
					<TableSkeleton rows={3} columns={3} />
				) : (
					<ProvidersList
						providers={list}
						isAdmin={isAdmin}
						currentSubject={me.data?.subject}
						testResults={testProvider.results}
						testingKind={testProvider.pendingKind}
						onConfigure={openEditor}
						onTest={testProvider.test}
						onDelete={openDelete}
					/>
				)}
			</SectionCard>
			<ProviderKeyDialog
				open={editorOpen}
				kind={editingKind}
				isReplacing={list.some((provider) => provider.kind === editingKind)}
				apiKey={values.apiKey}
				baseUrl={values.baseUrl}
				apiKeyError={fieldError('apiKey')}
				baseUrlError={fieldError('baseUrl')}
				serverError={saveProvider.errorMessage}
				isPending={saveProvider.isPending}
				onApiKeyChange={(value) => form.setFieldValue('apiKey', value)}
				onBaseUrlChange={(value) => form.setFieldValue('baseUrl', value)}
				onSubmit={() => {
					form.handleSubmit().catch(() => undefined)
				}}
				onClose={closeEditor}
			/>
			<DeleteProviderDialog
				open={deleteOpen}
				kind={deletingKind}
				isPending={deleteProvider.isPending}
				errorMessage={deleteProvider.errorMessage}
				onConfirm={confirmDelete}
				onClose={() => setDeleteOpen(false)}
			/>
		</>
	)
}
