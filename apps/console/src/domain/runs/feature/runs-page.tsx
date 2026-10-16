import { useForm, useStore } from '@tanstack/react-form'
import { Card, CardContent, CardHeader, CardTitle } from '#/components/ui/card'
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

	return (
		<section className="flex flex-col gap-6">
			<h1 className="text-2xl font-semibold">Runs</h1>
			<Card>
				<CardHeader>
					<CardTitle>Déposer un transcript</CardTitle>
				</CardHeader>
				<CardContent>
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
				</CardContent>
			</Card>
			{runs.isError ? (
				<p role="alert" className="text-sm text-destructive">
					Impossible de charger les runs.
				</p>
			) : runs.isPending ? (
				<p className="text-sm text-muted-foreground">Chargement…</p>
			) : (
				<RunsTable runs={runs.data.items} />
			)}
		</section>
	)
}
