import { describe, expect, it } from 'vitest'
import {
	documentErrorMessage,
	READER_MISSING_MESSAGE,
} from '#/domain/runs/document-errors'
import { ApiRequestError } from '#/lib/api-error'

describe('documentErrorMessage', () => {
	it('points a missing reader at the Models page', () => {
		expect(
			documentErrorMessage(
				new ApiRequestError(409, 'reader_not_configured', 'x'),
			),
		).toBe(READER_MISSING_MESSAGE)
		expect(READER_MISSING_MESSAGE).toContain('Paramètres › Modèles')
	})

	it('translates the size and format refusals', () => {
		expect(
			documentErrorMessage(new ApiRequestError(413, 'payload', 'big')),
		).toBe('Le fichier dépasse 25 Mo.')
		expect(documentErrorMessage(new ApiRequestError(415, 'media', 'no'))).toBe(
			'Ce fichier n’est pas un PDF valide.',
		)
	})

	it('carries the reader message on a 502 as it is', () => {
		expect(
			documentErrorMessage(
				new ApiRequestError(
					502,
					'bad_gateway',
					'Le service de lecture des PDF est injoignable.',
				),
			),
		).toBe('Le service de lecture des PDF est injoignable.')
	})

	it('falls back on a network message', () => {
		expect(documentErrorMessage(new TypeError('Failed to fetch'))).toMatch(
			/Impossible de joindre l’API/,
		)
	})
})
