import type { ComponentProps, FormEvent } from 'react'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Button } from '#/components/ui/button'
import { Input } from '#/components/ui/input'
import { Label } from '#/components/ui/label'
import { Textarea } from '#/components/ui/textarea'
import { MeetingDetails } from '#/domain/runs/ui/meeting-details'
import { TranscriptDropzone } from '#/domain/runs/ui/transcript-dropzone'

export type DropzoneProps = Omit<
	ComponentProps<typeof TranscriptDropzone>,
	'disabled'
>
export type DetailsProps = Omit<
	ComponentProps<typeof MeetingDetails>,
	'disabled'
>

interface Props {
	title: string
	body: string
	titleMaxLength: number
	bodyMaxLength: number
	titleError?: string
	bodyError?: string
	serverError?: string
	successMessage?: string
	isPending: boolean
	onTitleChange: (value: string) => void
	onBodyChange: (value: string) => void
	dropzone: DropzoneProps
	details: DetailsProps
	onSubmit: () => void
}

export function SubmitTranscriptForm({
	title,
	body,
	titleMaxLength,
	bodyMaxLength,
	titleError,
	bodyError,
	serverError,
	successMessage,
	isPending,
	onTitleChange,
	onBodyChange,
	dropzone,
	details,
	onSubmit,
}: Props) {
	function handleSubmit(event: FormEvent<HTMLFormElement>) {
		event.preventDefault()
		onSubmit()
	}

	return (
		<form onSubmit={handleSubmit} className="flex flex-col gap-4" noValidate>
			<div className="flex flex-col gap-1.5">
				<Label htmlFor="transcript-title">Titre</Label>
				<Input
					id="transcript-title"
					value={title}
					onChange={(event) => onTitleChange(event.target.value)}
					disabled={isPending}
					aria-invalid={Boolean(titleError)}
				/>
				<div className="flex min-h-4 justify-between gap-2 text-xs">
					<span
						role={titleError ? 'alert' : undefined}
						className="text-signal-danger"
					>
						{titleError}
					</span>
					<span className="ml-auto text-muted-foreground figures">
						{title.length} / {titleMaxLength}
					</span>
				</div>
			</div>
			<TranscriptDropzone {...dropzone} disabled={isPending} />
			<div className="flex flex-col gap-1.5">
				<Label htmlFor="transcript-body">Transcript</Label>
				<Textarea
					id="transcript-body"
					className="max-h-80 min-h-40 font-mono text-[13px] leading-relaxed"
					value={body}
					onChange={(event) => onBodyChange(event.target.value)}
					disabled={isPending}
					aria-invalid={Boolean(bodyError)}
				/>
				<div className="flex min-h-4 justify-between gap-2 text-xs">
					<span
						role={bodyError ? 'alert' : undefined}
						className="text-signal-danger"
					>
						{bodyError}
					</span>
					<span className="ml-auto text-muted-foreground figures">
						{body.length} / {bodyMaxLength}
					</span>
				</div>
			</div>
			<MeetingDetails {...details} disabled={isPending} />
			{serverError ? (
				<InlineAlert tone="danger">{serverError}</InlineAlert>
			) : null}
			{successMessage ? (
				<InlineAlert tone="success">{successMessage}</InlineAlert>
			) : null}
			<div className="flex items-center justify-end border-t pt-4">
				<Button type="submit" disabled={isPending}>
					{isPending ? 'Envoi…' : 'Lancer le run'}
				</Button>
			</div>
		</form>
	)
}
