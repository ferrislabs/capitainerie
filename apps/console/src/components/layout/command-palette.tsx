import { Search } from 'lucide-react'
import { Dialog as DialogPrimitive } from 'radix-ui'
import { useId, useState } from 'react'
import {
	filterCommands,
	type PaletteCommand,
} from '#/components/layout/palette-filter'
import { cn } from '#/lib/utils'

interface Props {
	open: boolean
	onOpenChange: (open: boolean) => void
	commands: ReadonlyArray<PaletteCommand>
	onRun: (command: PaletteCommand) => void
}

export function CommandPalette({ open, onOpenChange, commands, onRun }: Props) {
	const listId = useId()
	const [query, setQuery] = useState('')
	const [active, setActive] = useState(0)
	const shown = filterCommands(commands, query)
	const current = Math.min(active, Math.max(shown.length - 1, 0))

	function close(next: boolean) {
		if (!next) {
			setQuery('')
			setActive(0)
		}
		onOpenChange(next)
	}

	function run(command: PaletteCommand | undefined) {
		if (!command) return
		close(false)
		onRun(command)
	}

	return (
		<DialogPrimitive.Root open={open} onOpenChange={close}>
			<DialogPrimitive.Portal>
				<DialogPrimitive.Overlay className="fixed inset-0 z-50 bg-black/45" />
				<DialogPrimitive.Content
					aria-describedby={undefined}
					className="fixed top-[14vh] left-1/2 z-50 w-[calc(100%-2rem)] max-w-lg -translate-x-1/2 overflow-hidden rounded-lg border bg-popover text-popover-foreground shadow-float"
				>
					<DialogPrimitive.Title className="sr-only">
						Palette de commandes
					</DialogPrimitive.Title>
					<div className="flex items-center gap-2 border-b px-3">
						<Search
							aria-hidden="true"
							className="size-4 text-muted-foreground"
						/>
						<input
							// biome-ignore lint/a11y/noAutofocus: la palette s'ouvre pour être tapée
							autoFocus
							role="combobox"
							aria-expanded="true"
							aria-controls={listId}
							aria-activedescendant={
								shown[current] ? `${listId}-${shown[current].id}` : undefined
							}
							aria-label="Rechercher une page ou une action"
							placeholder="Aller à une page, lancer une action…"
							value={query}
							onChange={(event) => {
								setQuery(event.target.value)
								setActive(0)
							}}
							onKeyDown={(event) => {
								if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
									event.preventDefault()
									if (shown.length === 0) return
									const step = event.key === 'ArrowDown' ? 1 : -1
									setActive((current + step + shown.length) % shown.length)
								} else if (event.key === 'Enter') {
									event.preventDefault()
									run(shown[current])
								}
							}}
							className="h-11 min-w-0 flex-1 bg-transparent text-sm outline-none placeholder:text-muted-foreground"
						/>
					</div>
					<div
						id={listId}
						role="listbox"
						aria-label="Résultats"
						className="max-h-80 overflow-y-auto p-1.5"
					>
						{shown.length === 0 ? (
							<p className="px-2.5 py-6 text-center text-muted-foreground">
								Aucun résultat
							</p>
						) : (
							shown.map((command, index) => {
								const Icon = command.icon
								return (
									<div
										key={command.id}
										id={`${listId}-${command.id}`}
										role="option"
										tabIndex={-1}
										aria-selected={index === current}
										onMouseMove={() => setActive(index)}
										onClick={() => run(command)}
										onKeyDown={() => {}}
										className={cn(
											'flex h-8 cursor-pointer items-center gap-2.5 rounded-md px-2.5',
											index === current && 'bg-accent',
										)}
									>
										{Icon ? (
											<Icon
												aria-hidden="true"
												className="size-4 text-muted-foreground"
											/>
										) : null}
										<span className="flex-1 truncate">{command.label}</span>
										<span className="text-xs text-muted-foreground">
											{command.group}
										</span>
									</div>
								)
							})
						)}
					</div>
				</DialogPrimitive.Content>
			</DialogPrimitive.Portal>
		</DialogPrimitive.Root>
	)
}
