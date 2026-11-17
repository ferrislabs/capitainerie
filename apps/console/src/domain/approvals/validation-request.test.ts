import { describe, expect, it } from 'vitest'
import {
	buildValidationRequest,
	nextApprovalId,
	validationTitle,
} from '#/domain/approvals/validation-request'
import { DEMO_APPROVALS } from '#/domain/demo'

const input = {
	approvals: DEMO_APPROVALS,
	raison: 'NOVABANK GROUPE',
	sourceName: 'Sous-domaines',
	reasons: ['Effectif : inconnu', 'Titulaire RDAP : masqué'],
	requestedBy: 'archibald',
}

describe('validation request', () => {
	it('numbers the next approval after the existing ones', () => {
		expect(nextApprovalId(DEMO_APPROVALS)).toBe('APP-4')
		expect(nextApprovalId([])).toBe('APP-1')
	})

	it('builds a pending approval', () => {
		const a = buildValidationRequest(input)
		expect(a).toEqual({
			id: 'APP-4',
			title: validationTitle('NOVABANK GROUPE', 'Sous-domaines'),
			detail: 'Effectif : inconnu ; Titulaire RDAP : masqué',
			requestedBy: 'archibald',
			status: 'pending',
			at: "à l'instant",
			decidedBy: '',
			taskId: undefined,
		})
		expect(a?.title).toBe(
			'Valider le rapprochement : NOVABANK GROUPE (Sous-domaines)',
		)
	})

	it('returns null when already requested', () => {
		const a = buildValidationRequest(input)
		if (!a) throw new Error('expected a request')
		expect(
			buildValidationRequest({ ...input, approvals: [a, ...DEMO_APPROVALS] }),
		).toBeNull()
	})

	it('allows another dossier for the same source', () => {
		const a = buildValidationRequest(input)
		if (!a) throw new Error('expected a request')
		const b = buildValidationRequest({
			...input,
			approvals: [a],
			raison: 'AUTRE SAS',
		})
		expect(b?.id).toBe('APP-2')
	})
})
