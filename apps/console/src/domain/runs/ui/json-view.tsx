import { Check, Copy } from 'lucide-react'
import { Button } from '#/components/ui/button'
import {
	type JsonTokenKind,
	prettyJson,
	tokenizeJson,
} from '#/domain/runs/json-tokens'
import { cn } from '#/lib/utils'

const TOKEN_CLASSES: Record<JsonTokenKind, string> = {
	key: 'font-medium text-foreground',
	string: 'text-signal-success',
	number: 'text-signal-running',
	literal: 'text-muted-foreground italic',
	punct: 'text-muted-foreground',
	space: '',
}

interface Props {
	value: unknown
	copied: boolean
	onCopy: (text: string) => void
}

export function JsonView({ value, copied, onCopy }: Props) {
	const text = prettyJson(value)
	const tokens = tokenizeJson(text)
	let offset = 0

	return (
		<div className="relative">
			<Button
				type="button"
				variant="outline"
				size="sm"
				className="absolute top-2 right-2"
				onClick={() => onCopy(text)}
			>
				{copied ? (
					<Check aria-hidden className="size-3.5" />
				) : (
					<Copy aria-hidden className="size-3.5" />
				)}
				{copied ? 'Copié' : 'Copier'}
			</Button>
			<figure aria-label="Sortie au format JSON" className="m-0">
				<pre className="max-h-96 overflow-auto rounded-md border bg-muted/50 p-3 pr-24 font-mono text-xs leading-relaxed">
					<code>
						{tokens.map((token) => {
							const key = offset
							offset += token.text.length
							return (
								<span key={key} className={cn(TOKEN_CLASSES[token.kind])}>
									{token.text}
								</span>
							)
						})}
					</code>
				</pre>
			</figure>
		</div>
	)
}
