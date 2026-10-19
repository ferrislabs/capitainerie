import { useForm, useStore } from '@tanstack/react-form'
import { Link } from '@tanstack/react-router'
import { useState } from 'react'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { SectionCard } from '#/components/layout/section-card'
import { TableSkeleton } from '#/components/layout/table-skeleton'
import { useClients, useProjects } from '#/domain/clients/hooks/use-directory'
import {
	useGithubIntegration,
	useGithubRepositories,
} from '#/domain/integrations/hooks/use-github-integration'
import { READER_MISSING_MESSAGE } from '#/domain/runs/document-errors'
import { useFileDrop } from '#/domain/runs/hooks/use-file-drop'
import { useReadDocument } from '#/domain/runs/hooks/use-read-document'
import { useRuns } from '#/domain/runs/hooks/use-runs'
import { useSubmitTranscript } from '#/domain/runs/hooks/use-submit-transcript'
import { useTranscribe } from '#/domain/runs/hooks/use-transcribe'
import {
	chooseClient,
	chooseProject,
	intakeBody,
} from '#/domain/runs/intake-links'
import {
	importTranscript,
	isAudioFile,
	isPdfFile,
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
import { TRANSCRIBER_MISSING_MESSAGE } from '#/domain/runs/transcription-errors'
import { RunsTable } from '#/domain/runs/ui/runs-table'
import { SubmitTranscriptForm } from '#/domain/runs/ui/submit-transcript-form'
import { useModelAssignments } from '#/domain/settings/hooks/use-settings-queries'

interface ImportedState {
	name: string
	characters: number
	detected: number
	previousBody: string
	added: string[]
	transcribedBy?: string
	readBy?: string
}

export function RunsPage() {
	const runs = useRuns()
	const submit = useSubmitTranscript()
	const transcription = useTranscribe()
	const reading = useReadDocument()
	const assignments = useModelAssignments()
	const transcriberMissing =
		assignments.isSuccess && !assignments.data.assignments?.transcriber
	const readerMissing =
		assignments.isSuccess && !assignments.data.assignments?.reader
	const integration = useGithubIntegration(true)
	const repositories = useGithubRepositories(
		Boolean(integration.data?.installation),
	)
	const clients = useClients()
	const projects = useProjects()
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
			clientId: '',
			projectId: '',
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
					...intakeBody(value),
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
		if (isPdfFile(file.name)) {
			await handlePdf(file)
			return
		}
		if (isAudioFile(file.name)) {
			await handleAudio(file)
			return
		}
		let content: string
		try {
			content = new TextDecoder('utf-8', { fatal: true }).decode(
				await file.arrayBuffer(),
			)
		} catch {
			setFileError("Le fichier n'est pas en UTF-8.")
			return
		}
		const result = importTranscript(file.name, content)
		applyText(file.name, result.text, result.participants)
	}

	async function handleAudio(file: File) {
		if (transcriberMissing) {
			setFileError(TRANSCRIBER_MISSING_MESSAGE)
			return
		}
		setFileError(undefined)
		const outcome = await transcription.run(file)
		if (outcome.kind === 'cancelled') return
		if (outcome.kind === 'failed') {
			setFileError(outcome.message)
			return
		}
		applyText(file.name, outcome.text, [], { transcribedBy: outcome.model })
	}

	async function handlePdf(file: File) {
		if (readerMissing) {
			setFileError(READER_MISSING_MESSAGE)
			return
		}
		setFileError(undefined)
		const outcome = await reading.run(file)
		if (outcome.kind === 'cancelled') return
		if (outcome.kind === 'failed') {
			setFileError(outcome.message)
			return
		}
		applyText(file.name, outcome.text, [], { readBy: outcome.model })
	}

	function applyText(
		name: string,
		text: string,
		detected: string[],
		source: { transcribedBy?: string; readBy?: string } = {},
	) {
		if (text.trim() === '') {
			setFileError(
				source.readBy
					? 'Ce PDF ne contient aucun texte exploitable.'
					: source.transcribedBy
						? 'Cet audio ne contient aucune parole exploitable.'
						: 'Ce fichier ne contient aucun texte exploitable.',
			)
			return
		}
		const characters = [...text].length
		if (characters > BODY_MAX_LENGTH) {
			setFileError(
				`Le transcript ne doit pas dépasser ${BODY_MAX_LENGTH} caractères.`,
			)
			return
		}
		setFileError(undefined)
		const before = form.getFieldValue('participants')
		const merged = mergeParticipants(before, detected)
		const known = new Set(before.map((person) => person.toLowerCase()))
		const added = [
			...(imported?.added ?? []),
			...merged.filter((person) => !known.has(person.toLowerCase())),
		]
		setImported({
			name,
			characters,
			detected: detected.length,
			previousBody: imported
				? imported.previousBody
				: form.getFieldValue('body'),
			added,
			...(source.transcribedBy ? { transcribedBy: source.transcribedBy } : {}),
			...(source.readBy ? { readBy: source.readBy } : {}),
		})
		form.setFieldValue('body', text)
		if (form.getFieldValue('title').trim() === '') {
			form.setFieldValue('title', titleFromFileName(name))
		}
		form.setFieldValue('participants', merged)
	}

	function removeImport() {
		if (!imported) return
		form.setFieldValue('body', imported.previousBody)
		const added = new Set(imported.added.map((name) => name.toLowerCase()))
		form.setFieldValue(
			'participants',
			form
				.getFieldValue('participants')
				.filter((name) => !added.has(name.toLowerCase())),
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

	const drop = useFileDrop(
		handleFile,
		setFileError,
		submit.isPending || transcription.isPending || reading.isPending,
	)

	function handleSubmit() {
		form.handleSubmit().catch(() => undefined)
	}

	const projectList = projects.data?.items.map((item) => item.project) ?? []
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
					description="Texte brut ou fichier .txt, .md, .vtt, .srt ou .pdf, 200 000 caractères au plus."
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
						isTranscribing={transcription.isPending || reading.isPending}
						onTitleChange={(value) => form.setFieldValue('title', value)}
						onBodyChange={(value) => form.setFieldValue('body', value)}
						dropzone={{
							imported: imported && {
								name: imported.name,
								characters: imported.characters,
								participants: imported.detected,
								transcribedBy: imported.transcribedBy,
								readBy: imported.readBy,
							},
							error: fileError,
							transcribingFile: transcription.fileName,
							readingFile: reading.fileName,
							readerMissing,
							transcriberMissing,
							modelsLink: (
								<Link
									to="/settings/models"
									className="font-medium text-foreground underline underline-offset-2"
								>
									Choisir un transcripteur
								</Link>
							),
							readerLink: (
								<Link
									to="/settings/models"
									className="font-medium text-foreground underline underline-offset-2"
								>
									Choisir un lecteur de PDF
								</Link>
							),
							onCancel: () => {
								transcription.cancel()
								reading.cancel()
							},
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
							clients: clients.data?.items.map((item) => item.client),
							projects: projectList,
							clientId: values.clientId,
							projectId: values.projectId,
							onClientChange: (clientId) => {
								const next = chooseClient(projectList, values, clientId)
								form.setFieldValue('clientId', next.clientId)
								form.setFieldValue('projectId', next.projectId)
							},
							onProjectChange: (projectId) => {
								const next = chooseProject(projectList, values, projectId)
								form.setFieldValue('clientId', next.clientId)
								form.setFieldValue('projectId', next.projectId)
								if (next.repository) {
									form.setFieldValue('repository', next.repository)
								}
							},
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
