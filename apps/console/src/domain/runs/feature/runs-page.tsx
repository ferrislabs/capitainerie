import { useForm, useStore } from '@tanstack/react-form'
import { useState } from 'react'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { SectionCard } from '#/components/layout/section-card'
import { TableSkeleton } from '#/components/layout/table-skeleton'
import {
	useGithubIntegration,
	useGithubRepositories,
} from '#/domain/integrations/hooks/use-github-integration'
import { useFileDrop } from '#/domain/runs/hooks/use-file-drop'
import { useRuns } from '#/domain/runs/hooks/use-runs'
import { useSubmitTranscript } from '#/domain/runs/hooks/use-submit-transcript'
import {
	importTranscript,
	titleFromFileName,
	validateTranscriptFile,
} from '#/domain/runs/transcript-import'
import {
	addParticipants,
	BODY_MAX_LENGTH,
	mergeParticipants,
	TITLE_MAX_LENGTH,
	validateBody,
	validateParticipants,
	validateRepository,
	validateTitle,
} from '#/domain/runs/transcript-rules'
import { RunsTable } from '#/domain/runs/ui/runs-table'
import { SubmitTranscriptForm } from '#/domain/runs/ui/submit-transcript-form'

interface ImportedState {
	name: string
	characters: number
	participants: string[]
}

export function RunsPage() {
	const runs = useRuns()
	const submit = useSubmitTranscript()
	const integration = useGithubIntegration(true)
	const repositories = useGithubRepositories(
		Boolean(integration.data?.installation),
	)
	const [imported, setImported] = useState<ImportedState>()
	const [fileError, setFileError] = useState<string>()
	const [participantDraft, setParticipantDraft] = useState('')

	const form = useForm({
		defaultValues: {
			title: '',
			body: '',
			meetingDate: '',
			participants: [] as string[],
			repository: '',
		},
		validators: {
			onChange: ({ value }) => {
				const title = validateTitle(value.title)
				const body = validateBody(value.body)
				const participants = validateParticipants(value.participants)
				const repository = validateRepository(value.repository)
				return title || body || participants || repository
					? { fields: { title, body, participants, repository } }
					: undefined
			},
		},
		onSubmit: async ({ value, formApi }) => {
			const participants = addParticipants(value.participants, participantDraft)
			const repository = value.repository.trim()
			await submit.mutateAsync({
				body: {
					title: value.title.trim(),
					body: value.body,
					...(value.meetingDate ? { meeting_date: value.meetingDate } : {}),
					...(participants.length > 0 ? { participants } : {}),
					...(repository ? { repository } : {}),
				},
			})
			formApi.reset()
			setImported(undefined)
			setFileError(undefined)
			setParticipantDraft('')
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
		const problem = validateTranscriptFile(file)
		if (problem) {
			setFileError(problem)
			return
		}
		const result = importTranscript(file.name, await file.text())
		if (result.text.trim() === '') {
			setFileError('Ce fichier ne contient aucun texte exploitable.')
			return
		}
		setFileError(undefined)
		form.setFieldValue('body', result.text)
		if (form.getFieldValue('title').trim() === '') {
			form.setFieldValue('title', titleFromFileName(file.name))
		}
		form.setFieldValue(
			'participants',
			mergeParticipants(
				form.getFieldValue('participants'),
				result.participants,
			),
		)
		setImported({
			name: file.name,
			characters: result.text.length,
			participants: result.participants,
		})
	}

	function removeImport() {
		if (!imported) return
		form.setFieldValue('body', '')
		form.setFieldValue(
			'participants',
			form
				.getFieldValue('participants')
				.filter((name) => !imported.participants.includes(name)),
		)
		setImported(undefined)
		setFileError(undefined)
	}

	function commitParticipantDraft() {
		if (participantDraft === '') return
		form.setFieldValue(
			'participants',
			addParticipants(form.getFieldValue('participants'), participantDraft),
		)
		setParticipantDraft('')
	}

	function changeParticipantDraft(value: string) {
		const comma = value.lastIndexOf(',')
		if (comma < 0) {
			setParticipantDraft(value)
			return
		}
		form.setFieldValue(
			'participants',
			addParticipants(
				form.getFieldValue('participants'),
				value.slice(0, comma),
			),
		)
		setParticipantDraft(value.slice(comma + 1))
	}

	const drop = useFileDrop(handleFile, submit.isPending)

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
					description="Texte brut ou fichier .txt, .md, .vtt ou .srt, 200 000 caractères au plus."
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
						dropzone={{
							imported: imported && {
								name: imported.name,
								characters: imported.characters,
								participants: imported.participants.length,
							},
							error: fileError,
							isDragging: drop.isDragging,
							dropHandlers: drop.handlers,
							onFileSelected: handleFile,
							onRemove: removeImport,
						}}
						details={{
							meetingDate: values.meetingDate,
							participants: values.participants,
							participantDraft,
							repository: values.repository,
							repositories: repositories.data?.repositories,
							participantsError: validateParticipants(values.participants),
							repositoryError: validateRepository(values.repository),
							onMeetingDateChange: (value) =>
								form.setFieldValue('meetingDate', value),
							onParticipantDraftChange: changeParticipantDraft,
							onParticipantDraftCommit: commitParticipantDraft,
							onParticipantRemove: (name) =>
								form.setFieldValue(
									'participants',
									values.participants.filter((item) => item !== name),
								),
							onRepositoryChange: (value) =>
								form.setFieldValue('repository', value),
						}}
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
