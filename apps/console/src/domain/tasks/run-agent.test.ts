import { describe, expect, it } from 'vitest'
import { pullRequestLink } from '#/domain/tasks/pull-request'
import {
	canRunAgent,
	describeRunError,
	initialRunPlan,
	mentionedRepository,
	runRequest,
	writesHint,
} from '#/domain/tasks/run-agent'
import { ApiRequestError } from '#/lib/api-error'

const GRANTS = ['acme/web', 'acme/web-admin']

describe('mentionedRepository', () => {
	it('finds the granted repository the task names, whole name only', () => {
		expect(
			mentionedRepository(GRANTS, 'Audite le dépôt ACME/Web-Admin stp'),
		).toBe('acme/web-admin')
		expect(mentionedRepository(GRANTS, 'Audite le dépôt acme/web.')).toBe(
			'acme/web',
		)
		expect(mentionedRepository(GRANTS, 'Audite acme/website')).toBeUndefined()
		expect(mentionedRepository(GRANTS, 'Rien à voir')).toBeUndefined()
	})
})

describe('initialRunPlan', () => {
	it('picks the repository the task names', () => {
		expect(initialRunPlan(GRANTS, true, 'Audite acme/web-admin')).toEqual({
			target: 'repository',
			repository: 'acme/web-admin',
			baseBranch: '',
			allowWrites: false,
		})
	})

	it('keeps connectors when the agent has some and the task names no repository', () => {
		expect(initialRunPlan(GRANTS, true, 'Relance les clients')).toMatchObject({
			target: 'connectors',
			repository: 'acme/web',
		})
	})

	it('works on a repository when the agent has no connector', () => {
		expect(initialRunPlan(GRANTS, false, '')).toMatchObject({
			target: 'repository',
			repository: 'acme/web',
		})
		expect(initialRunPlan([], false, '').target).toBe('connectors')
	})
})

describe('runRequest', () => {
	const plan = initialRunPlan(GRANTS, false, '')

	it('sends the repository and a trimmed base branch only when set', () => {
		expect(runRequest(plan, 3)).toEqual({
			expected_version: 3,
			allow_writes: false,
			repository: 'acme/web',
		})
		expect(
			runRequest({ ...plan, baseBranch: ' develop ', allowWrites: true }, 3),
		).toEqual({
			expected_version: 3,
			allow_writes: true,
			repository: 'acme/web',
			base_branch: 'develop',
		})
	})

	it('omits the repository for a connectors run', () => {
		expect(runRequest({ ...plan, target: 'connectors' }, 4)).toEqual({
			expected_version: 4,
			allow_writes: false,
		})
	})
})

describe('writesHint', () => {
	it('explains what writes mean for each target', () => {
		const plan = initialRunPlan(GRANTS, false, '')
		expect(writesHint(plan)).toBe(
			"Audit en lecture seule : l'agent lit le code et produit un rapport.",
		)
		expect(writesHint({ ...plan, allowWrites: true })).toBe(
			"L'agent pousse une branche et ouvre une PR brouillon.",
		)
		expect(writesHint({ ...plan, target: 'connectors' })).toMatch(
			/service connecté/,
		)
	})
})

describe('canRunAgent', () => {
	it('needs an assignee and an open card, not a connector', () => {
		expect(canRunAgent('todo', true)).toBe(true)
		expect(canRunAgent('todo', false)).toBe(false)
		expect(canRunAgent('done', true)).toBe(false)
		expect(canRunAgent('rejected', true)).toBe(false)
	})
})

describe('describeRunError', () => {
	const error = (status: number, code: string, message = code) =>
		new ApiRequestError(status, code, message)

	it.each([
		[
			error(422, 'repository_not_granted'),
			'agent_repositories',
			/n'a pas accès à ce dépôt/,
		],
		[
			error(422, 'invalid', 'base_branch is not a valid branch name'),
			'message',
			/branche de base est invalide/,
		],
		[error(409, 'github_not_connected'), 'github', /GitHub n'est pas connecté/],
		[
			error(409, 'conflict', 'github_not_connected'),
			'github',
			/GitHub n'est pas connecté/,
		],
		[
			error(403, 'forbidden', 'repository_not_allowed'),
			'github',
			/ajoute-le à l'installation/,
		],
		[
			error(409, 'conflict', 'execution_already_active'),
			'message',
			/déjà en cours/,
		],
		[
			error(409, 'no_provider', 'Aucun fournisseur Anthropic'),
			'no_provider',
			/Aucun fournisseur Anthropic/,
		],
		[
			error(409, 'agent_identity_not_active'),
			'message',
			/identité FerrisKey de l'agent n'est pas encore prête/,
		],
		[error(422, 'agent_paused'), 'message', /en pause/],
		[error(422, 'agent_archived'), 'message', /archivé/],
		[
			error(422, 'agent_without_connector'),
			'agent_repositories',
			/ses dépôts GitHub/,
		],
	])('maps %o', (input, kind, message) => {
		const info = describeRunError(input)
		expect(info.kind).toBe(kind)
		expect(info.message).toMatch(message)
		expect(info.stale).toBe(false)
	})

	it('shows why the executor is disabled', () => {
		const info = describeRunError(
			error(
				409,
				'executor_disabled',
				'les exécutions sur un dépôt sont désactivées : EXECUTOR_NAMESPACE manquant',
			),
		)
		expect(info.message).toBe(
			'Les exécutions sur un dépôt sont désactivées sur ce serveur : EXECUTOR_NAMESPACE manquant.',
		)
	})

	it('marks a version conflict as stale', () => {
		expect(describeRunError(error(409, 'conflict', 'version')).stale).toBe(true)
	})
})

describe('pullRequestLink', () => {
	it('extracts the first GitHub pull request of a comment', () => {
		expect(
			pullRequestLink(
				'Claude Code a ouvert une pull request en brouillon : https://github.com/acme/web/pull/12\n\nRésumé',
			),
		).toEqual({
			url: 'https://github.com/acme/web/pull/12',
			repository: 'acme/web',
			number: 12,
		})
		expect(pullRequestLink('Aucun changement.')).toBeNull()
	})
})
