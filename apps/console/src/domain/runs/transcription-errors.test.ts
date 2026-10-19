import { describe, expect, it } from 'vitest'
import {
	TRANSCRIBER_MISSING_MESSAGE,
	transcriptionErrorMessage,
} from '#/domain/runs/transcription-errors'
import { ApiRequestError } from '#/lib/api-error'

describe('transcriptionErrorMessage', () => {
	it('points a missing transcriber at the Models page', () => {
		expect(
			transcriptionErrorMessage(
				new ApiRequestError(409, 'transcriber_not_configured', 'x'),
			),
		).toBe(TRANSCRIBER_MISSING_MESSAGE)
		expect(TRANSCRIBER_MISSING_MESSAGE).toContain('Paramètres › Modèles')
	})

	it('translates the size and format refusals', () => {
		expect(
			transcriptionErrorMessage(new ApiRequestError(413, 'payload', 'big')),
		).toBe('Le fichier dépasse 25 Mo.')
		expect(
			transcriptionErrorMessage(new ApiRequestError(415, 'media', 'pdf')),
		).toMatch(/Format audio non pris en charge/)
	})

	it('carries the provider message on a 502', () => {
		expect(
			transcriptionErrorMessage(
				new ApiRequestError(502, 'bad_gateway', 'invalid api key'),
			),
		).toBe('Le fournisseur n’a pas pu transcrire ce fichier : invalid api key')
	})

	it('falls back on the server message, then on a network message', () => {
		expect(
			transcriptionErrorMessage(new ApiRequestError(403, 'forbidden', 'Non')),
		).toBe('Non')
		expect(transcriptionErrorMessage(new TypeError('Failed to fetch'))).toMatch(
			/Impossible de joindre l’API/,
		)
	})
})
