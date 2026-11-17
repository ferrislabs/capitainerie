import { type FormEvent, useState } from 'react'
import { Button } from '#/components/ui/button'
import { AddRow, CTL } from '#/domain/sources/ui/mockup'
import { cn } from '#/lib/utils'

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
		<div>
			<ul className="m-0 flex list-none flex-wrap gap-1.5 px-3 py-2.5">
				{items.length > 0 ? (
					items.map((item) => (
						<li
							key={item}
							className="inline-flex items-center gap-1 rounded-full border bg-background py-0.5 pr-1 pl-[9px] font-mono text-[12px]"
						>
							{item}
							<button
								type="button"
								aria-label={`Retirer ${item}`}
								onClick={() => onRemove(item)}
								className="size-[18px] cursor-pointer rounded-full border-0 bg-transparent p-0 leading-none text-faint hover:bg-secondary hover:text-signal-danger"
							>
								×
							</button>
						</li>
					))
				) : (
					<li className="text-muted-foreground">Aucun</li>
				)}
			</ul>
			<form onSubmit={submit}>
				<AddRow>
					<input
						className={cn(CTL, 'flex-1 font-mono text-[11.5px]')}
						aria-label={addLabel}
						placeholder={placeholder}
						value={draft}
						onChange={(e) => setDraft(e.target.value)}
					/>
					<Button type="submit" variant="outline">
						Ajouter
					</Button>
				</AddRow>
			</form>
		</div>
	)
}
