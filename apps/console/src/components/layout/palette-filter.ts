import type { LucideIcon } from 'lucide-react'

export interface PaletteCommand {
	id: string
	group: string
	label: string
	icon?: LucideIcon
	to?: string
	run?: () => void
}

function fold(text: string): string {
	return text.normalize('NFD').replace(/[̀-ͯ]/g, '').toLowerCase()
}

/** Garde les commandes dont le libellé ou le groupe contient la saisie, sans accent ni casse. */
export function filterCommands(
	commands: ReadonlyArray<PaletteCommand>,
	query: string,
): ReadonlyArray<PaletteCommand> {
	const needle = fold(query.trim())
	if (!needle) return commands
	return commands.filter((c) => fold(`${c.group} ${c.label}`).includes(needle))
}
