import { Link } from '@tanstack/react-router'
import { ArrowUp, Loader2, Mic, Square } from 'lucide-react'
import type { KeyboardEvent } from 'react'
import { Button } from '#/components/ui/button'
import { Textarea } from '#/components/ui/textarea'
import type { CaptainModel } from '#/domain/chat/hooks/use-captain-model'
import type { VoiceState } from '#/domain/chat/hooks/use-voice-input'
import { cn } from '#/lib/utils'

export interface VoiceControls {
	state: VoiceState
	supported: boolean
	onToggle: () => void
	onCancel: () => void
}

interface Props {
	value: string
	onChange: (value: string) => void
	onSend: () => void
	onAbort?: () => void
	label: string
	placeholder: string
	streaming?: boolean
	sending?: boolean
	disabled?: boolean
	autoFocus?: boolean
	voice: VoiceControls
	model: CaptainModel
	className?: string
}

const VOICE_LABELS: Record<VoiceState, string> = {
	idle: 'Dicter',
	recording: 'Arrêter la dictée',
	transcribing: 'Annuler la transcription',
}

function ModelLabel({ model }: { model: CaptainModel }) {
	if (model.kind === 'loading') return null
	if (model.kind === 'unknown') {
		return (
			<Link
				to="/settings/models"
				className="truncate text-xs text-muted-foreground underline-offset-4 hover:underline"
			>
				Modèle non choisi
			</Link>
		)
	}
	return (
		<span
			className="truncate font-mono text-xs text-muted-foreground"
			title="Modèle du Capitaine"
		>
			{model.model}
		</span>
	)
}

export function Composer({
	value,
	onChange,
	onSend,
	onAbort,
	label,
	placeholder,
	streaming = false,
	sending = false,
	disabled = false,
	autoFocus = false,
	voice,
	model,
	className,
}: Props) {
	const canSend = !disabled && !streaming && !sending && value.trim() !== ''

	function handleKeyDown(event: KeyboardEvent<HTMLTextAreaElement>) {
		if (event.key === 'Escape') {
			if (streaming && onAbort) {
				event.preventDefault()
				onAbort()
			} else if (voice.state !== 'idle') {
				event.preventDefault()
				voice.onCancel()
			}
			return
		}
		if (
			event.key === 'Enter' &&
			!event.shiftKey &&
			!event.nativeEvent.isComposing
		) {
			event.preventDefault()
			if (canSend) onSend()
		}
	}

	const voiceBusy = voice.state !== 'idle'

	return (
		<div className={cn('flex flex-col gap-1.5', className)}>
			<div className="flex flex-col gap-1 rounded-lg border border-input bg-card p-2 focus-within:border-ring focus-within:ring-[3px] focus-within:ring-ring/25">
				<Textarea
					aria-label={label}
					placeholder={placeholder}
					value={value}
					disabled={disabled}
					autoFocus={autoFocus}
					rows={2}
					onChange={(event) => onChange(event.target.value)}
					onKeyDown={handleKeyDown}
					className="max-h-64 min-h-12 resize-none border-0 bg-transparent px-1 py-1 shadow-none focus-visible:ring-0"
				/>
				<div className="flex items-center gap-2">
					{voice.supported ? (
						<Button
							type="button"
							variant="ghost"
							size="icon-sm"
							aria-label={VOICE_LABELS[voice.state]}
							aria-pressed={voice.state === 'recording'}
							disabled={disabled || streaming}
							onClick={voice.onToggle}
							className={cn(
								voice.state === 'recording' &&
									'bg-signal-danger-soft text-signal-danger hover:bg-signal-danger-soft hover:text-signal-danger',
							)}
						>
							{voice.state === 'transcribing' ? (
								<Loader2 aria-hidden="true" className="animate-spin" />
							) : (
								<Mic aria-hidden="true" />
							)}
						</Button>
					) : null}
					{voiceBusy ? (
						<output className="text-xs text-muted-foreground">
							{voice.state === 'recording'
								? 'Enregistrement… cliquez pour terminer.'
								: 'Transcription…'}
						</output>
					) : null}
					<span className="ml-auto flex min-w-0 items-center gap-3">
						<ModelLabel model={model} />
						{streaming ? (
							<Button
								type="button"
								size="sm"
								variant="outline"
								onClick={onAbort}
							>
								<Square aria-hidden="true" className="size-3 fill-current" />
								Interrompre
							</Button>
						) : (
							<Button
								type="button"
								size="icon-sm"
								aria-label="Envoyer"
								disabled={!canSend}
								onClick={onSend}
							>
								{sending ? (
									<Loader2 aria-hidden="true" className="animate-spin" />
								) : (
									<ArrowUp aria-hidden="true" />
								)}
							</Button>
						)}
					</span>
				</div>
			</div>
			<p className="px-1 text-[11px] text-muted-foreground">
				Entrée envoie · Maj+Entrée va à la ligne · Échap interrompt
			</p>
		</div>
	)
}
