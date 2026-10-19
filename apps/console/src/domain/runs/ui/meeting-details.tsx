import { ChevronRight, X } from 'lucide-react'
import type { KeyboardEvent } from 'react'
import type { Schemas } from '#/api/api.client'
import { Input } from '#/components/ui/input'
import { Label } from '#/components/ui/label'

interface Props {
	meetingDate: string
	participants: readonly string[]
	participantDraft: string
	repository: string
	repositories?: ReadonlyArray<Schemas.RepositoryResponse>
	participantsError?: string
	repositoryError?: string
	disabled: boolean
	onMeetingDateChange: (value: string) => void
	onParticipantDraftChange: (value: string) => void
	onParticipantDraftCommit: () => void
	onParticipantRemove: (name: string) => void
	onRepositoryChange: (value: string) => void
}

function FieldError({ id, message }: { id: string; message?: string }) {
	return (
		<span
			id={id}
			role={message ? 'alert' : undefined}
			className="min-h-4 text-xs text-signal-danger"
		>
			{message}
		</span>
	)
}

export function MeetingDetails({
	meetingDate,
	participants,
	participantDraft,
	repository,
	repositories,
	participantsError,
	repositoryError,
	disabled,
	onMeetingDateChange,
	onParticipantDraftChange,
	onParticipantDraftCommit,
	onParticipantRemove,
	onRepositoryChange,
}: Props) {
	function handleKeyDown(event: KeyboardEvent<HTMLInputElement>) {
		if (event.key === 'Enter') {
			event.preventDefault()
			onParticipantDraftCommit()
		} else if (
			event.key === 'Backspace' &&
			participantDraft === '' &&
			participants.length > 0
		) {
			onParticipantRemove(participants[participants.length - 1])
		}
	}

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
				<div className="flex flex-col gap-1.5">
					<Label htmlFor="meeting-date">Date de la réunion</Label>
					<Input
						id="meeting-date"
						type="date"
						className="w-44"
						value={meetingDate}
						onChange={(event) => onMeetingDateChange(event.target.value)}
						disabled={disabled}
					/>
				</div>
				<div className="flex flex-col gap-1.5">
					<Label htmlFor="meeting-participants">Participants</Label>
					<div className="flex flex-wrap items-center gap-1.5 rounded-md border border-input bg-card px-2 py-1.5 focus-within:border-ring focus-within:ring-[3px] focus-within:ring-ring/25">
						{participants.map((name) => (
							<span
								key={name}
								className="inline-flex items-center gap-1 rounded-sm bg-secondary px-1.5 py-0.5 text-xs font-medium"
							>
								{name}
								<button
									type="button"
									aria-label={`Retirer ${name}`}
									className="rounded-sm text-muted-foreground hover:text-foreground disabled:pointer-events-none"
									disabled={disabled}
									onClick={() => onParticipantRemove(name)}
								>
									<X aria-hidden="true" className="size-3" />
								</button>
							</span>
						))}
						<input
							id="meeting-participants"
							className="min-w-32 flex-1 bg-transparent py-0.5 text-sm outline-none placeholder:text-muted-foreground"
							placeholder="Nom, puis Entrée ou virgule"
							value={participantDraft}
							onChange={(event) => onParticipantDraftChange(event.target.value)}
							onKeyDown={handleKeyDown}
							onBlur={onParticipantDraftCommit}
							disabled={disabled}
							aria-describedby="meeting-participants-error"
						/>
					</div>
					<FieldError
						id="meeting-participants-error"
						message={participantsError}
					/>
				</div>
				<div className="flex flex-col gap-1.5">
					<Label htmlFor="meeting-repository">Dépôt par défaut</Label>
					{repositories && repositories.length > 0 ? (
						<select
							id="meeting-repository"
							className="h-8 w-full rounded-md border border-input bg-card px-3 font-mono text-xs text-foreground focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/25 focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-60"
							value={repository}
							onChange={(event) => onRepositoryChange(event.target.value)}
							disabled={disabled}
						>
							<option value="">Aucun</option>
							{repositories.map((item) => (
								<option key={item.full_name} value={item.full_name}>
									{item.full_name}
								</option>
							))}
						</select>
					) : (
						<Input
							id="meeting-repository"
							className="font-mono text-xs"
							placeholder="propriétaire/nom"
							value={repository}
							onChange={(event) => onRepositoryChange(event.target.value)}
							disabled={disabled}
							aria-invalid={Boolean(repositoryError)}
							aria-describedby="meeting-repository-error"
						/>
					)}
					<FieldError id="meeting-repository-error" message={repositoryError} />
					<span className="text-xs text-muted-foreground">
						Proposé par défaut quand vous confiez une carte de cette réunion à
						Claude Code.
					</span>
				</div>
			</div>
		</details>
	)
}
