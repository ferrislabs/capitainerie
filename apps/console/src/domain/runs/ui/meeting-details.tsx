import { ChevronRight, X } from 'lucide-react'
import type { KeyboardEvent } from 'react'
import type { Schemas } from '#/api/api.client'
import { Combobox } from '#/components/ui/combobox'
import { DateInput } from '#/components/ui/date-input'
import { Field, useFieldControl } from '#/components/ui/field'
import { FIELD_BOX, FIELD_PADDING, FIELD_TEXT } from '#/components/ui/field-box'
import { Input } from '#/components/ui/input'
import { projectsOf } from '#/domain/runs/intake-links'
import { cn } from '#/lib/utils'

interface Props {
	meetingDate: string
	participants: readonly string[]
	participantDraft: string
	repository: string
	repositories?: ReadonlyArray<Schemas.RepositoryResponse>
	clients?: ReadonlyArray<Schemas.ClientResponse>
	projects?: ReadonlyArray<Schemas.ProjectResponse>
	clientId?: string
	projectId?: string
	onClientChange?: (clientId: string) => void
	onProjectChange?: (projectId: string) => void
	participantsError?: string
	repositoryError?: string
	disabled: boolean
	onMeetingDateChange: (value: string) => void
	onParticipantDraftChange: (value: string) => void
	onParticipantDraftCommit: () => void
	onParticipantRemove: (name: string) => void
	onRepositoryChange: (value: string) => void
}

interface ParticipantsInputProps {
	participants: readonly string[]
	draft: string
	disabled: boolean
	onDraftChange: (value: string) => void
	onCommit: () => void
	onRemove: (name: string) => void
}

function ParticipantsInput({
	participants,
	draft,
	disabled,
	onDraftChange,
	onCommit,
	onRemove,
}: ParticipantsInputProps) {
	const control = useFieldControl({ disabled })

	function handleKeyDown(event: KeyboardEvent<HTMLInputElement>) {
		if (event.key === 'Enter') {
			event.preventDefault()
			onCommit()
		} else if (
			event.key === 'Backspace' &&
			draft === '' &&
			participants.length > 0
		) {
			onRemove(participants[participants.length - 1])
		}
	}

	return (
		<div
			data-invalid={control.invalid}
			data-disabled={control.disabled}
			className={cn(
				FIELD_BOX,
				FIELD_PADDING.md,
				FIELD_TEXT.md,
				'flex-wrap gap-1.5 py-1',
			)}
		>
			{participants.map((name) => (
				<span
					key={name}
					className="inline-flex h-5 items-center gap-1 rounded-sm bg-secondary pr-0.5 pl-1.5 text-xs font-medium"
				>
					{name}
					<button
						type="button"
						aria-label={`Retirer ${name}`}
						className="flex size-4 items-center justify-center rounded-sm text-muted-foreground hover:bg-accent hover:text-accent-foreground disabled:pointer-events-none"
						disabled={control.disabled}
						onClick={() => onRemove(name)}
					>
						<X aria-hidden="true" className="size-3" />
					</button>
				</span>
			))}
			<input
				id={control.id}
				className="h-6 min-w-32 flex-1 bg-transparent outline-none placeholder:text-muted-foreground"
				placeholder="Nom, puis Entrée ou virgule"
				value={draft}
				onChange={(event) => onDraftChange(event.target.value)}
				onKeyDown={handleKeyDown}
				onBlur={onCommit}
				disabled={control.disabled}
				aria-invalid={control.invalid || undefined}
				aria-describedby={control.describedBy}
			/>
		</div>
	)
}

export function MeetingDetails({
	meetingDate,
	participants,
	participantDraft,
	repository,
	repositories,
	clients = [],
	projects = [],
	clientId = '',
	projectId = '',
	onClientChange,
	onProjectChange,
	participantsError,
	repositoryError,
	disabled,
	onMeetingDateChange,
	onParticipantDraftChange,
	onParticipantDraftCommit,
	onParticipantRemove,
	onRepositoryChange,
}: Props) {
	return (
		<details className="group rounded-md border">
			<summary className="flex cursor-pointer list-none items-center gap-2 px-3 py-2 text-[13px] font-medium [&::-webkit-details-marker]:hidden">
				<ChevronRight
					aria-hidden="true"
					className="size-4 text-muted-foreground transition-transform group-open:rotate-90"
				/>
				Détails de la réunion
				<span className="font-normal text-muted-foreground">(optionnel)</span>
			</summary>
			<div className="flex flex-col gap-4 border-t px-3 py-3">
				<Field label="Date de la réunion" id="meeting-date">
					<DateInput
						className="w-44"
						value={meetingDate}
						onChange={(event) => onMeetingDateChange(event.target.value)}
						disabled={disabled}
					/>
				</Field>
				<Field
					label="Participants"
					id="meeting-participants"
					error={participantsError}
				>
					<ParticipantsInput
						participants={participants}
						draft={participantDraft}
						disabled={disabled}
						onDraftChange={onParticipantDraftChange}
						onCommit={onParticipantDraftCommit}
						onRemove={onParticipantRemove}
					/>
				</Field>
				{onClientChange && onProjectChange ? (
					<div className="grid gap-4 sm:grid-cols-2">
						<Field label="Client" id="meeting-client">
							<Combobox
								disabled={disabled}
								items={clients.map((client) => ({
									value: client.id,
									label: client.name,
								}))}
								value={clientId}
								noneLabel="Détecter automatiquement"
								onValueChange={onClientChange}
							/>
						</Field>
						<Field label="Projet" id="meeting-project">
							<Combobox
								disabled={disabled}
								items={projectsOf(projects, clientId).map((project) => ({
									value: project.id,
									label: project.name,
								}))}
								value={projectId}
								noneLabel="Détecter automatiquement"
								onValueChange={onProjectChange}
							/>
						</Field>
						<span className="text-xs text-muted-foreground sm:col-span-2">
							Un choix ici s’applique à toutes les cartes du run et remplace la
							détection. Choisir un projet remplit le client et le dépôt.
						</span>
					</div>
				) : null}
				<Field
					label="Dépôt par défaut"
					id="meeting-repository"
					error={repositoryError}
					hint="Proposé par défaut quand vous confiez une carte de cette réunion à Claude Code."
				>
					{repositories && repositories.length > 0 ? (
						<Combobox
							className="font-mono text-xs"
							disabled={disabled}
							items={repositories.map((item) => ({
								value: item.full_name,
								label: item.full_name,
							}))}
							value={repository}
							noneLabel="Aucun"
							onValueChange={onRepositoryChange}
						/>
					) : (
						<Input
							className="font-mono text-xs"
							placeholder="propriétaire/nom"
							value={repository}
							onChange={(event) => onRepositoryChange(event.target.value)}
							disabled={disabled}
						/>
					)}
				</Field>
			</div>
		</details>
	)
}
