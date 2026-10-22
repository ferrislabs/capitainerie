import type { ComponentProps, FormEvent } from 'react'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Button } from '#/components/ui/button'
import { Field } from '#/components/ui/field'
import { Input } from '#/components/ui/input'
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
	isTranscribing?: boolean
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
	isTranscribing = false,
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
			<Field
				label="Titre"
				id="transcript-title"
				count={title.length}
				max={titleMaxLength}
				error={titleError}
			>
				<Input
					value={title}
					onChange={(event) => onTitleChange(event.target.value)}
					disabled={isPending}
				/>
			</Field>
			<TranscriptDropzone {...dropzone} disabled={isPending} />
			<Field
				label="Transcript"
				id="transcript-body"
				count={body.length}
				max={bodyMaxLength}
				error={bodyError}
			>
				<Textarea
					className="max-h-80 min-h-40 font-mono text-[13px] leading-relaxed"
					value={body}
					onChange={(event) => onBodyChange(event.target.value)}
					disabled={isPending || isTranscribing}
				/>
			</Field>
			<MeetingDetails {...details} disabled={isPending} />
			{serverError ? (
				<InlineAlert tone="danger">{serverError}</InlineAlert>
			) : null}
			{successMessage ? (
				<InlineAlert tone="success">{successMessage}</InlineAlert>
			) : null}
			<div className="flex items-center justify-end border-t pt-4">
				<Button type="submit" disabled={isPending || isTranscribing}>
					{isPending ? 'Envoi…' : 'Lancer le run'}
				</Button>
			</div>
		</form>
	)
}
