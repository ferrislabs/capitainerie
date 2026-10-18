import { FileUp } from 'lucide-react'
import type { ChangeEvent, FormEvent } from 'react'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Button } from '#/components/ui/button'
import { Input } from '#/components/ui/input'
import { Label } from '#/components/ui/label'
import { Textarea } from '#/components/ui/textarea'

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
	onFileSelected: (file: File) => void
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
	onFileSelected,
	onSubmit,
}: Props) {
	function handleSubmit(event: FormEvent<HTMLFormElement>) {
		event.preventDefault()
		onSubmit()
	}

	function handleFile(event: ChangeEvent<HTMLInputElement>) {
		const file = event.target.files?.[0]
		if (file) onFileSelected(file)
		event.target.value = ''
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
			<div className="flex flex-col gap-1.5">
				<Label htmlFor="transcript-file">
					Importer un fichier .txt{' '}
					<span className="font-normal text-muted-foreground">(optionnel)</span>
				</Label>
				<div className="flex items-center gap-3">
					<input
						id="transcript-file"
						type="file"
						className="peer sr-only"
						accept=".txt,text/plain"
						onChange={handleFile}
						disabled={isPending}
					/>
					<label
						htmlFor="transcript-file"
						className="inline-flex h-8 cursor-pointer items-center gap-2 rounded-md border border-input/70 bg-card px-3 text-[13px] font-medium transition-colors duration-150 hover:bg-accent peer-focus-visible:outline-2 peer-focus-visible:outline-offset-2 peer-focus-visible:outline-ring peer-disabled:pointer-events-none peer-disabled:opacity-50"
					>
						<FileUp aria-hidden="true" className="size-4" />
						Choisir un fichier
					</label>
					<span className="text-xs text-muted-foreground">
						Son contenu remplace le transcript.
					</span>
				</div>
			</div>
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
