import { X } from 'lucide-react'
import { type FormEvent, useState } from 'react'
import { Button } from '#/components/ui/button'
import { TextInput } from '#/components/ui/text-input'

interface Props {
	items: ReadonlyArray<string>
	placeholder: string
	addLabel: string
	onRemove: (item: string) => void
	onAdd: (item: string) => void
}

export function PatternList({
	items,
	placeholder,
	addLabel,
	onRemove,
	onAdd,
}: Props) {
	const [draft, setDraft] = useState('')
	function submit(event: FormEvent) {
		event.preventDefault()
		const value = draft.trim()
		if (!value) return
		onAdd(value)
		setDraft('')
	}
	return (
		<div className="flex flex-col gap-3">
			<ul className="flex flex-wrap gap-2">
				{items.length > 0 ? (
					items.map((item) => (
						<li
							key={item}
							className="inline-flex items-center gap-1 rounded-md bg-muted py-0.5 pr-0.5 pl-2 font-mono"
						>
							{item}
							<Button
								type="button"
								variant="ghost"
								size="icon-sm"
								aria-label={`Retirer ${item}`}
								onClick={() => onRemove(item)}
							>
								<X aria-hidden="true" />
							</Button>
						</li>
					))
				) : (
					<li className="text-muted-foreground">Aucun</li>
				)}
			</ul>
			<form onSubmit={submit} className="flex gap-2">
				<TextInput
					className="font-mono"
					aria-label={addLabel}
					placeholder={placeholder}
					value={draft}
					onChange={(e) => setDraft(e.target.value)}
				/>
				<Button type="submit" variant="outline">
					Ajouter
				</Button>
			</form>
		</div>
	)
}
