import type { ChangeEvent, FormEvent } from 'react'
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
			<div className="flex flex-col gap-2">
				<Label htmlFor="transcript-title">Titre</Label>
				<Input
					id="transcript-title"
					value={title}
					onChange={(event) => onTitleChange(event.target.value)}
					disabled={isPending}
					aria-invalid={Boolean(titleError)}
				/>
				<div className="flex justify-between gap-2 text-xs">
					<span
						role={titleError ? 'alert' : undefined}
						className="text-destructive"
					>
						{titleError}
					</span>
					<span className="text-muted-foreground">
						{title.length} / {titleMaxLength}
					</span>
				</div>
			</div>
			<div className="flex flex-col gap-2">
				<Label htmlFor="transcript-body">Transcript</Label>
				<Textarea
					id="transcript-body"
					className="min-h-40"
					value={body}
					onChange={(event) => onBodyChange(event.target.value)}
					disabled={isPending}
					aria-invalid={Boolean(bodyError)}
				/>
				<div className="flex justify-between gap-2 text-xs">
					<span
						role={bodyError ? 'alert' : undefined}
						className="text-destructive"
					>
						{bodyError}
					</span>
					<span className="text-muted-foreground">
						{body.length} / {bodyMaxLength}
					</span>
				</div>
			</div>
			<div className="flex flex-col gap-2">
				<Label htmlFor="transcript-file">
					Importer un fichier .txt (optionnel)
				</Label>
				<Input
					id="transcript-file"
					type="file"
					accept=".txt,text/plain"
					onChange={handleFile}
					disabled={isPending}
				/>
			</div>
			{serverError ? (
				<p role="alert" className="text-sm text-destructive">
					{serverError}
				</p>
			) : null}
			{successMessage ? (
				<output className="text-sm text-muted-foreground">
					{successMessage}
				</output>
			) : null}
			<div>
				<Button type="submit" disabled={isPending}>
					{isPending ? 'Envoi…' : 'Lancer le run'}
				</Button>
			</div>
		</form>
	)
}
