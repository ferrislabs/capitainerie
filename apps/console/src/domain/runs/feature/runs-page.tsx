import { useForm, useStore } from '@tanstack/react-form'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { SectionCard } from '#/components/layout/section-card'
import { TableSkeleton } from '#/components/layout/table-skeleton'
import { useRuns } from '#/domain/runs/hooks/use-runs'
import { useSubmitTranscript } from '#/domain/runs/hooks/use-submit-transcript'
import {
	BODY_MAX_LENGTH,
	TITLE_MAX_LENGTH,
	validateBody,
	validateTitle,
} from '#/domain/runs/transcript-rules'
import { RunsTable } from '#/domain/runs/ui/runs-table'
import { SubmitTranscriptForm } from '#/domain/runs/ui/submit-transcript-form'

export function RunsPage() {
	const runs = useRuns()
	const submit = useSubmitTranscript()

	const form = useForm({
		defaultValues: { title: '', body: '' },
		validators: {
			onChange: ({ value }) => {
				const title = validateTitle(value.title)
				const body = validateBody(value.body)
				return title || body ? { fields: { title, body } } : undefined
			},
		},
		onSubmit: async ({ value, formApi }) => {
			await submit.mutateAsync({
				body: { title: value.title.trim(), body: value.body },
			})
			formApi.reset()
		},
	})

	const values = useStore(form.store, (state) => state.values)
	const fieldMeta = useStore(form.store, (state) => state.fieldMeta)
	const attempts = useStore(form.store, (state) => state.submissionAttempts)

	function visibleError(name: 'title' | 'body') {
		const meta = fieldMeta[name]
		if (!meta || (!meta.isTouched && attempts === 0)) return undefined
		const first = meta.errors[0]
		return typeof first === 'string' ? first : undefined
	}

	async function handleFile(file: File) {
		form.setFieldValue('body', await file.text())
	}

	function handleSubmit() {
		form.handleSubmit().catch(() => undefined)
	}

	const runCount = runs.data?.items.length

	return (
		<Page>
			<PageHeader
				title="Runs"
				description="Déposez le transcript d'une réunion : les agents en extraient les actions, les dédoublonnent et les proposent au Kanban."
			/>
			<div className="grid items-start gap-5 xl:grid-cols-[minmax(18rem,24rem)_minmax(0,1fr)]">
				<SectionCard
					title="Déposer un transcript"
					description="Texte brut ou fichier .txt, 200 000 caractères au plus."
				>
					<SubmitTranscriptForm
						title={values.title}
						body={values.body}
						titleMaxLength={TITLE_MAX_LENGTH}
						bodyMaxLength={BODY_MAX_LENGTH}
						titleError={visibleError('title')}
						bodyError={visibleError('body')}
						serverError={submit.error?.message}
						successMessage={submit.isSuccess ? 'Run lancé' : undefined}
						isPending={submit.isPending}
						onTitleChange={(value) => form.setFieldValue('title', value)}
						onBodyChange={(value) => form.setFieldValue('body', value)}
						onFileSelected={handleFile}
						onSubmit={handleSubmit}
					/>
				</SectionCard>
				<SectionCard
					title="Derniers runs"
					description={
						runCount === undefined
							? 'Les 20 runs les plus récents.'
							: `${runCount} run${runCount > 1 ? 's' : ''} parmi les plus récents.`
					}
					bare
				>
					{runs.isError ? (
						<InlineAlert tone="danger">
							Impossible de charger les runs.
						</InlineAlert>
					) : runs.isPending ? (
						<TableSkeleton rows={6} columns={4} />
					) : (
						<RunsTable runs={runs.data.items} />
					)}
				</SectionCard>
			</div>
		</Page>
	)
}
