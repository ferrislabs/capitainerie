import { Check, Copy } from 'lucide-react'
import type { MouseEvent } from 'react'
import { useCopyToClipboard } from '#/hooks/use-copy-to-clipboard'

interface Props {
	value: string
}

export function CopyIdButton({ value }: Props) {
	const { copied, copy } = useCopyToClipboard()
	const Icon = copied ? Check : Copy
	const label = copied ? 'Identifiant copié' : 'Copier l’identifiant'

	function handleClick(event: MouseEvent<HTMLButtonElement>) {
		event.stopPropagation()
		copy(value)
	}

	return (
		<>
			<button
				type="button"
				onClick={handleClick}
				aria-label={label}
				title={label}
				className="flex size-7 shrink-0 items-center justify-center rounded-md text-muted-foreground transition-colors duration-150 hover:bg-accent hover:text-accent-foreground focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring"
			>
				<Icon aria-hidden="true" className="size-3.5" />
			</button>
			<output aria-live="polite" className="sr-only">
				{copied ? 'Copié' : ''}
			</output>
		</>
	)
}
