import { FileText, FileUp, X } from 'lucide-react'
import type { ChangeEvent, DragEvent } from 'react'
import { Button } from '#/components/ui/button'
import { ACCEPTED_EXTENSIONS } from '#/domain/runs/transcript-import'
import { cn } from '#/lib/utils'

export interface ImportedFile {
	name: string
	characters: number
	participants: number
}

interface DropHandlers {
	onDragEnter: (event: DragEvent<HTMLElement>) => void
	onDragOver: (event: DragEvent<HTMLElement>) => void
	onDragLeave: (event: DragEvent<HTMLElement>) => void
	onDrop: (event: DragEvent<HTMLElement>) => void
}

interface Props {
	imported?: ImportedFile
	error?: string
	isDragging: boolean
	disabled: boolean
	dropHandlers: DropHandlers
	onFileSelected: (file: File) => void
	onRemove: () => void
}

function plural(count: number, one: string, many: string) {
	return `${count.toLocaleString('fr-FR')} ${count > 1 ? many : one}`
}

export function TranscriptDropzone({
	imported,
	error,
	isDragging,
	disabled,
	dropHandlers,
	onFileSelected,
	onRemove,
}: Props) {
	function handleChange(event: ChangeEvent<HTMLInputElement>) {
		const file = event.target.files?.[0]
		if (file) onFileSelected(file)
		event.target.value = ''
	}

	return (
		<div className="flex flex-col gap-2">
			<input
				id="transcript-file"
				type="file"
				className="peer sr-only"
				accept={ACCEPTED_EXTENSIONS.join(',')}
				onChange={handleChange}
				disabled={disabled}
			/>
			<label
				htmlFor="transcript-file"
				{...dropHandlers}
				className={cn(
					'flex cursor-pointer flex-col items-center gap-1.5 rounded-md border border-dashed border-input px-4 py-5 text-center transition-colors duration-150 hover:bg-accent peer-focus-visible:outline-2 peer-focus-visible:outline-offset-2 peer-focus-visible:outline-ring peer-disabled:pointer-events-none peer-disabled:opacity-50',
					isDragging && 'border-signal-running bg-signal-running-soft',
				)}
			>
				<FileUp aria-hidden="true" className="size-5 text-muted-foreground" />
				<span className="text-[13px] font-medium">
					Glissez un fichier ici, ou cliquez pour en choisir un
				</span>
				<span className="text-xs text-muted-foreground">
					.txt, .md, .vtt ou .srt, 5 Mo au plus. Son contenu remplace le
					transcript.
				</span>
			</label>
			{error ? (
				<p role="alert" className="text-xs text-signal-danger">
					{error}
				</p>
			) : null}
			{imported ? (
				<div className="flex items-center justify-between gap-2 rounded-md border bg-muted/40 px-3 py-1.5 text-xs">
					<span className="flex min-w-0 items-center gap-2">
						<FileText
							aria-hidden="true"
							className="size-3.5 shrink-0 text-muted-foreground"
						/>
						<span className="truncate font-medium">{imported.name}</span>
						<span className="shrink-0 text-muted-foreground figures">
							{plural(imported.characters, 'caractère', 'caractères')} ·{' '}
							{plural(
								imported.participants,
								'participant détecté',
								'participants détectés',
							)}
						</span>
					</span>
					<Button
						type="button"
						size="sm"
						variant="ghost"
						disabled={disabled}
						onClick={onRemove}
					>
						<X aria-hidden="true" className="size-3.5" />
						Retirer
					</Button>
				</div>
			) : null}
		</div>
	)
}
