import type { Announcements } from '@dnd-kit/core'
import type { Schemas } from '#/api/api.client'
import { canDrop } from '#/domain/kanban/can-drop'
import { COLUMN_LABELS, type Column } from '#/domain/kanban/columns'
import { columnOfData } from '#/domain/kanban/drag-data'

export const SCREEN_READER_INSTRUCTIONS =
	'Pour prendre une carte, appuyez sur Espace. Déplacez-la avec les flèches, posez-la avec Espace, annulez avec Échap.'

interface Ref {
	id: string | number
	data: { current?: unknown }
}

export function createAnnouncements(
	cards: ReadonlyArray<Schemas.CardResponse>,
): Announcements {
	function title(id: string | number): string {
		const card = cards.find((candidate) => candidate.id === id)
		return `« ${card?.title ?? 'carte'} »`
	}

	function origin(id: string | number): Column | undefined {
		return cards.find((candidate) => candidate.id === id)?.column
	}

	function target(over: Ref | null): Column | undefined {
		return over ? columnOfData(over.data.current) : undefined
	}

	return {
		onDragStart({ active }) {
			const from = origin(active.id)
			return from
				? `Carte ${title(active.id)} prise dans la colonne ${COLUMN_LABELS[from]}.`
				: `Carte ${title(active.id)} prise.`
		},
		onDragOver({ active, over }) {
			const from = origin(active.id)
			const to = target(over)
			if (!from || !to) return `Carte ${title(active.id)} hors des colonnes.`
			if (!canDrop(from, to)) {
				return `Carte ${title(active.id)} : déplacement vers la colonne ${COLUMN_LABELS[to]} impossible.`
			}
			return `Carte ${title(active.id)} au-dessus de la colonne ${COLUMN_LABELS[to]}.`
		},
		onDragEnd({ active, over }) {
			const from = origin(active.id)
			const to = target(over)
			if (from && to && canDrop(from, to)) {
				return `Carte ${title(active.id)} déposée dans la colonne ${COLUMN_LABELS[to]}.`
			}
			return from
				? `Carte ${title(active.id)} remise dans la colonne ${COLUMN_LABELS[from]}.`
				: `Carte ${title(active.id)} reposée.`
		},
		onDragCancel({ active }) {
			const from = origin(active.id)
			return from
				? `Déplacement annulé. Carte ${title(active.id)} remise dans la colonne ${COLUMN_LABELS[from]}.`
				: 'Déplacement annulé.'
		},
	}
}
