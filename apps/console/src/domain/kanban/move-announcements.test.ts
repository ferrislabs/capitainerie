import { describe, expect, it } from 'vitest'
import type { Schemas } from '#/api/api.client'
import {
	createAnnouncements,
	SCREEN_READER_INSTRUCTIONS,
} from '#/domain/kanban/move-announcements'

const CARDS: Schemas.CardResponse[] = [
	{
		id: 'c1',
		title: 'Écrire le compte rendu',
		description: '',
		column: 'todo',
		provenance: null,
		created_by_run: null,
		created_by_subject: 'ada',
		created_by_agent: null,
		version: 1,
		created_at: '2026-10-16T09:30:00Z',
		updated_at: '2026-10-16T09:30:00Z',
	},
]

function event(over?: unknown) {
	return {
		active: { id: 'c1', data: { current: { column: 'todo' } } },
		over: over
			? { id: 'x', data: { current: over }, rect: {}, disabled: false }
			: null,
	} as never
}

describe('createAnnouncements', () => {
	const announce = createAnnouncements(CARDS)

	it('explains the keyboard gestures in French', () => {
		expect(SCREEN_READER_INSTRUCTIONS).toContain('Espace')
		expect(SCREEN_READER_INSTRUCTIONS).toContain('flèches')
		expect(SCREEN_READER_INSTRUCTIONS).toContain('Échap')
	})

	it('announces the pick-up with the title and the column', () => {
		expect(announce.onDragStart?.(event())).toBe(
			'Carte « Écrire le compte rendu » prise dans la colonne À faire.',
		)
	})

	it('announces the column under the card, or that it cannot go there', () => {
		expect(announce.onDragOver?.(event({ column: 'done' }))).toBe(
			'Carte « Écrire le compte rendu » au-dessus de la colonne Terminé.',
		)
		expect(announce.onDragOver?.(event({ column: 'todo' }))).toBe(
			"Carte « Écrire le compte rendu » au-dessus de sa colonne d'origine.",
		)
		expect(announce.onDragOver?.(event())).toBe(
			'Carte « Écrire le compte rendu » hors des colonnes.',
		)
	})

	it('announces the drop and the return to the origin', () => {
		expect(announce.onDragEnd?.(event({ column: 'in_progress' }))).toBe(
			'Carte « Écrire le compte rendu » déposée dans la colonne En cours.',
		)
		expect(announce.onDragEnd?.(event({ column: 'todo' }))).toBe(
			'Carte « Écrire le compte rendu » remise dans la colonne À faire.',
		)
		expect(announce.onDragCancel?.(event())).toBe(
			'Déplacement annulé. Carte « Écrire le compte rendu » remise dans la colonne À faire.',
		)
	})
})
