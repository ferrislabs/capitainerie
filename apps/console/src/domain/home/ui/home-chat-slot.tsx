import type { CaptainModel } from '#/domain/chat/hooks/use-captain-model'
import { Composer, type VoiceControls } from '#/domain/chat/ui/composer'

interface Props {
	value: string
	onChange: (value: string) => void
	onSubmit: () => void
	isPending: boolean
	errorMessage?: string
	voice: VoiceControls
	voiceError?: string
	model: CaptainModel
}

export function HomeChatSlot({
	value,
	onChange,
	onSubmit,
	isPending,
	errorMessage,
	voice,
	voiceError,
	model,
}: Props) {
	return (
		<section className="flex flex-col gap-3">
			<h2
				id="home-chat-title"
				className="text-[15px] leading-tight font-semibold"
			>
				Que lance-t-on ?
			</h2>
			<Composer
				label="Que lance-t-on ?"
				placeholder="Ajoute un README au dépôt test pour Acme…"
				value={value}
				onChange={onChange}
				onSend={onSubmit}
				sending={isPending}
				disabled={isPending}
				voice={voice}
				model={model}
			/>
			{errorMessage || voiceError ? (
				<p role="alert" className="text-[13px] text-signal-danger">
					{errorMessage ?? voiceError}
				</p>
			) : null}
		</section>
	)
}
