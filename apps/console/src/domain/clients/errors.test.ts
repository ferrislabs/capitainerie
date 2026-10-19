import { describe, expect, it } from 'vitest'
import { directoryErrorMessage } from '#/domain/clients/errors'
import { ApiRequestError } from '#/lib/api-error'

describe('directoryErrorMessage', () => {
	it('explains a duplicate name', () => {
		expect(
			directoryErrorMessage(new ApiRequestError(409, 'conflict', 'x')),
		).toBe('Ce nom est déjà utilisé.')
	})

	it('keeps the server message for a refused input', () => {
		expect(
			directoryErrorMessage(
				new ApiRequestError(422, 'invalid', 'the name is empty'),
			),
		).toBe('the name is empty')
	})

	it('says nothing without an error', () => {
		expect(directoryErrorMessage(null)).toBeUndefined()
	})
})
