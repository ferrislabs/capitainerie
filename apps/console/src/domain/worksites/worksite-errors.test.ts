import { describe, expect, it } from 'vitest'
import {
	ISSUES_PERMISSION_MESSAGE,
	issuesErrorMessage,
	launchErrorMessage,
} from '#/domain/worksites/worksite-errors'
import { ApiRequestError } from '#/lib/api-error'

function failure(status: number, message: string) {
	return new ApiRequestError(status, message, message)
}

describe('launchErrorMessage', () => {
	it('is silent without an error', () => {
		expect(launchErrorMessage(null)).toBeUndefined()
	})

	it.each([
		['github_not_connected', 409, /GitHub n'est pas connecté/],
		['repository_not_allowed', 403, /n'a pas accès à ce dépôt/],
		['issue_already_imported', 409, /Rien n'a été lancé/],
	])('explains %s', (key, status, expected) => {
		expect(launchErrorMessage(failure(status, key))).toMatch(expected)
	})

	it('points to the missing GitHub permission', () => {
		expect(
			launchErrorMessage(failure(422, 'github_app_issues_permission_missing')),
		).toBe(ISSUES_PERMISSION_MESSAGE)
	})

	it('names a closed issue and says nothing was launched', () => {
		expect(launchErrorMessage(failure(422, 'issue #7 is not open'))).toBe(
			"issue #7 n'est plus ouverte. Rien n'a été lancé.",
		)
	})

	it('tells a vanished project and a refusal apart', () => {
		expect(launchErrorMessage(failure(404, 'not found'))).toBe(
			"Ce projet n'existe plus.",
		)
		expect(launchErrorMessage(failure(403, 'denied'))).toMatch(/pas le droit/)
		expect(launchErrorMessage(new Error('network'))).toBe(
			'Impossible de lancer les chantiers.',
		)
	})
})

describe('issuesErrorMessage', () => {
	it('separates a missing permission from a foreign repository', () => {
		expect(
			issuesErrorMessage(failure(422, 'github_app_issues_permission_missing')),
		).toBe(ISSUES_PERMISSION_MESSAGE)
		expect(issuesErrorMessage(failure(403, 'repository_not_allowed'))).toMatch(
			/n'a pas accès à ce dépôt/,
		)
		expect(issuesErrorMessage(new Error('x'))).toMatch(/Impossible de charger/)
	})
})
