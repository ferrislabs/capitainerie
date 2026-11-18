import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { renderAtPath } from '#/components/layout/test-router'
import { CURRENT_PROJECT_STORAGE_KEY } from '#/domain/worksites/current-project'
import { WorksitesPage } from '#/domain/worksites/feature/worksites-page'
import { clearAuth } from '#/store/auth.store'
import { mockApi, withQueryClient } from '#/test-utils/api-mock'
import { chooseValue } from '#/test-utils/combobox'

function project(id: string, name: string, repository: string | null) {
	return {
		id,
		client: { id: 'cl1', name: 'Acme' },
		name,
		aliases: [],
		repository,
		description: '',
		archived: false,
		created_by: 'ada',
		created_at: '2026-10-16T09:30:00Z',
		updated_at: '2026-10-16T09:30:00Z',
	}
}

const PROJECTS = {
	items: [
		{ open_cards: 0, project: project('p1', 'Site vitrine', 'acme/web') },
		{ open_cards: 0, project: project('p2', 'API', null) },
	],
}

const WORKSITES = {
	items: [
		{
			card_id: 'c1aaaaaa-0000-4000-8000-000000000001',
			title: 'Ajouter un CHANGELOG',
			column: 'in_progress',
			created_at: '2026-11-03T09:00:00Z',
			repository: 'acme/web',
			run_id: 'r1',
			run_status: 'running',
		},
		{
			card_id: 'c2bbbbbb-0000-4000-8000-000000000002',
			title: 'Crash on empty input',
			column: 'in_progress',
			created_at: '2026-11-03T08:00:00Z',
			repository: 'acme/web',
			issue_number: 7,
			issue_url: 'https://github.com/acme/web/issues/7',
			run_id: 'r2',
			run_status: 'queued',
		},
	],
}

const CONNECTED = {
	app: { slug: 'capi' },
	installation: { account_login: 'acme' },
}

const REPOSITORIES = {
	repositories: [
		{
			full_name: 'acme/web',
			name: 'web',
			owner: 'acme',
			private: false,
			html_url: 'https://github.com/acme/web',
		},
		{
			full_name: 'acme/api',
			name: 'api',
			owner: 'acme',
			private: true,
			html_url: 'https://github.com/acme/api',
		},
	],
}

const ISSUES = {
	issues: [
		{
			number: 7,
			title: 'Crash on empty input',
			html_url: 'https://github.com/acme/web/issues/7',
			labels: [],
			card_id: 'c2',
		},
		{
			number: 11,
			title: 'Dark mode',
			html_url: 'https://github.com/acme/web/issues/11',
			labels: ['ui'],
		},
	],
}

type Routes = Parameters<typeof mockApi>[0]

const BASE: Routes = {
	'GET /api/v1/projects': PROJECTS,
	'GET /api/v1/projects/p1/worksites': WORKSITES,
	'GET /api/v1/projects/p2/worksites': { items: [] },
	'GET /api/v1/integrations/github': CONNECTED,
	'GET /api/v1/integrations/github/repositories': REPOSITORIES,
	'GET /api/v1/integrations/github/issues': ISSUES,
}

async function open(routes: Routes = {}) {
	const fetchMock = mockApi({ ...BASE, ...routes })
	await renderAtPath(withQueryClient(<WorksitesPage />), '/worksites')
	return fetchMock
}

function launchBodies(fetchMock: ReturnType<typeof mockApi>): unknown[] {
	return fetchMock.mock.calls
		.filter(([, init]) => init?.method === 'POST')
		.map(([, init]) => JSON.parse(String(init?.body)))
}

afterEach(() => {
	vi.unstubAllGlobals()
	clearAuth()
	window.localStorage.clear()
})

describe('WorksitesPage', () => {
	it('asks for a project first, then lists its worksites with their state', async () => {
		const user = userEvent.setup()
		await open()

		expect(await screen.findByText('Choisis un projet')).toBeTruthy()
		expect(
			screen.getByRole('button', { name: 'Nouveau chantier' }),
		).toHaveProperty('disabled', true)

		await chooseValue(user, await screen.findByLabelText('Projet'), 'p1')

		const title = await screen.findByRole('link', {
			name: 'Ajouter un CHANGELOG',
		})
		expect(title.getAttribute('href')).toBe(
			'/tasks/c1aaaaaa-0000-4000-8000-000000000001',
		)
		expect(screen.getByText(/2 chantiers sur Site vitrine/)).toBeTruthy()
		expect(screen.getByText('En cours')).toBeTruthy()
		expect(screen.getByText('En file')).toBeTruthy()
		expect(
			screen.getByRole('link', { name: /Issue #7/ }).getAttribute('href'),
		).toBe('https://github.com/acme/web/issues/7')
		expect(window.localStorage.getItem(CURRENT_PROJECT_STORAGE_KEY)).toBe('p1')
	})

	it('comes back on the project chosen last time', async () => {
		window.localStorage.setItem(CURRENT_PROJECT_STORAGE_KEY, 'p1')
		await open()

		expect(await screen.findByText('Ajouter un CHANGELOG')).toBeTruthy()
	})

	it('forgets a remembered project that no longer exists', async () => {
		window.localStorage.setItem(CURRENT_PROJECT_STORAGE_KEY, 'gone')
		await open()

		expect(await screen.findByText('Choisis un projet')).toBeTruthy()
	})

	it('says a project has no worksite yet', async () => {
		window.localStorage.setItem(CURRENT_PROJECT_STORAGE_KEY, 'p2')
		await open()

		expect(await screen.findByText('Aucun chantier')).toBeTruthy()
	})

	it('points to the projects when there is none', async () => {
		await open({ 'GET /api/v1/projects': { items: [] } })

		expect(await screen.findByText('Aucun projet')).toBeTruthy()
		expect(
			screen
				.getByRole('link', { name: 'Voir les projets' })
				.getAttribute('href'),
		).toBe('/projects')
	})

	it('lists the repositories the GitHub app can reach', async () => {
		window.localStorage.setItem(CURRENT_PROJECT_STORAGE_KEY, 'p1')
		await open()

		const section = (
			await screen.findByRole('heading', { name: 'Dépôts GitHub accessibles' })
		).closest('section') as HTMLElement
		expect(await within(section).findByText('acme/api')).toBeTruthy()
		expect(within(section).getByText(/Installation sur acme/)).toBeTruthy()
		expect(within(section).getByText('2 dépôts accessibles')).toBeTruthy()
	})

	it('says when GitHub is not connected', async () => {
		window.localStorage.setItem(CURRENT_PROJECT_STORAGE_KEY, 'p1')
		await open({
			'GET /api/v1/integrations/github': { app: null, installation: null },
		})

		expect(
			await screen.findByText(/GitHub n'est pas connecté à l'organisation/),
		).toBeTruthy()
	})

	it('launches a prompt on the project default repository', async () => {
		window.localStorage.setItem(CURRENT_PROJECT_STORAGE_KEY, 'p1')
		const fetchMock = await open({
			'POST /api/v1/projects/p1/worksites': {
				status: 202,
				body: { worksites: [{ run_id: 'r9', card: { id: 'c9' } }] },
			},
		})
		const user = userEvent.setup()
		await screen.findByText('Ajouter un CHANGELOG')

		await user.click(screen.getByRole('button', { name: 'Nouveau chantier' }))
		const dialog = await screen.findByRole('dialog')
		await user.click(within(dialog).getByLabelText('Prompt 1'))
		await user.paste('Passer les tests en CI')
		await user.click(
			await within(dialog).findByRole('button', { name: 'Lancer le chantier' }),
		)

		await waitFor(() =>
			expect(launchBodies(fetchMock)).toEqual([
				{
					repository: 'acme/web',
					prompts: ['Passer les tests en CI'],
					issues: [],
				},
			]),
		)
		expect(await screen.findByText(/1 chantier lancé/)).toBeTruthy()
		await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull())
	})

	it('launches several issues and prompts in one call', async () => {
		window.localStorage.setItem(CURRENT_PROJECT_STORAGE_KEY, 'p1')
		const fetchMock = await open({
			'POST /api/v1/projects/p1/worksites': {
				status: 202,
				body: {
					worksites: [
						{ run_id: 'r8', card: { id: 'c8' } },
						{ run_id: 'r9', card: { id: 'c9' } },
					],
				},
			},
		})
		const user = userEvent.setup()
		await screen.findByText('Ajouter un CHANGELOG')

		await user.click(screen.getByRole('button', { name: 'Nouveau chantier' }))
		const dialog = await screen.findByRole('dialog')
		await user.click(
			await within(dialog).findByRole('checkbox', { name: /Dark mode/ }),
		)
		await user.click(within(dialog).getByLabelText('Prompt 1'))
		await user.paste('Mettre à jour le README')
		await user.click(
			within(dialog).getByRole('button', { name: 'Lancer 2 chantiers' }),
		)

		await waitFor(() =>
			expect(launchBodies(fetchMock)).toEqual([
				{
					repository: 'acme/web',
					prompts: ['Mettre à jour le README'],
					issues: [11],
				},
			]),
		)
		expect(await screen.findByText(/2 chantiers lancés/)).toBeTruthy()
	})

	it('keeps the dialog open and says why when the server refuses', async () => {
		window.localStorage.setItem(CURRENT_PROJECT_STORAGE_KEY, 'p1')
		await open({
			'POST /api/v1/projects/p1/worksites': {
				status: 409,
				body: {
					code: 'issue_already_imported',
					message: 'issue already imported',
					status: 409,
				},
			},
		})
		const user = userEvent.setup()
		await screen.findByText('Ajouter un CHANGELOG')

		await user.click(screen.getByRole('button', { name: 'Nouveau chantier' }))
		const dialog = await screen.findByRole('dialog')
		await user.click(within(dialog).getByLabelText('Prompt 1'))
		await user.paste('x')
		await user.click(
			await within(dialog).findByRole('button', { name: 'Lancer le chantier' }),
		)

		expect(await within(dialog).findByText(/a déjà un chantier/)).toBeTruthy()
		expect(screen.getByRole('dialog')).toBeTruthy()
	})

	it('explains the missing issues permission without blocking prompts', async () => {
		window.localStorage.setItem(CURRENT_PROJECT_STORAGE_KEY, 'p1')
		await open({
			'GET /api/v1/integrations/github/issues': {
				status: 422,
				body: {
					code: 'github_app_issues_permission_missing',
					message: 'missing permission',
					status: 422,
				},
			},
		})
		const user = userEvent.setup()
		await screen.findByText('Ajouter un CHANGELOG')

		await user.click(screen.getByRole('button', { name: 'Nouveau chantier' }))
		const dialog = await screen.findByRole('dialog')

		expect(await within(dialog).findByText(/Issues : Read-only/)).toBeTruthy()
		expect(within(dialog).getByLabelText('Prompt 1')).toBeTruthy()
	})
})
