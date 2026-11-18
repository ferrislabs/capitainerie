import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { renderAtPath } from '#/components/layout/test-router'
import { SkillsPage } from '#/domain/skills/feature/skills-page'
import { skill } from '#/domain/skills/test-fixtures'
import { clearAuth } from '#/store/auth.store'
import { mockApi, withQueryClient } from '#/test-utils/api-mock'
import { DEV, LEGAL, teamAgent } from '#/test-utils/team-agents'

const EXTRA = ['/agents/$agentId/{-$tab}']

const HUMANIZER = skill('humanizer', {
	name: 'Humanizer',
	description: 'Écriture naturelle',
	content: '# Humanizer\nÉcris simplement.',
	used_by: 2,
})
const GITLAB = skill('gitlab-mr', {
	name: 'GitLab MR',
	source: 'https://skills.example.com/gitlab-mr.md',
})

type Routes = Parameters<typeof mockApi>[0]

async function open(routes: Routes = {}) {
	const fetchMock = mockApi({
		'GET /api/v1/skills': { items: [HUMANIZER, GITLAB] },
		'GET /api/v1/team/agents': {
			items: [
				{ ...DEV, skills: ['humanizer'] },
				{ ...LEGAL, skills: ['humanizer', 'fix'] },
				teamAgent('a-other', 'Autre'),
			],
		},
		...routes,
	})
	await renderAtPath(withQueryClient(<SkillsPage />), '/skills', EXTRA)
	return fetchMock
}

function calls(fetchMock: ReturnType<typeof mockApi>, method: string) {
	return fetchMock.mock.calls
		.filter(([, init]) => init?.method === method)
		.map(([url, init]) => ({
			url: new URL(String(url)),
			body: init?.body ? JSON.parse(String(init.body)) : undefined,
		}))
}

afterEach(() => {
	vi.unstubAllGlobals()
	clearAuth()
})

describe('SkillsPage', () => {
	it('lists the real skills without demo banner', async () => {
		await open()
		expect(
			screen.getByRole('heading', { name: 'Compétences', level: 1 }),
		).toBeTruthy()
		expect(await screen.findByText('Humanizer')).toBeTruthy()
		expect(screen.getAllByRole('listitem')).toHaveLength(2)
		expect(screen.getByText('2 agents')).toBeTruthy()
		expect(screen.getByText('skills.example.com')).toBeTruthy()
		expect(screen.getByText(/2 compétences disponibles/)).toBeTruthy()
		expect(screen.queryByText(/démonstration/i)).toBeNull()
	})

	it('filters by search', async () => {
		const user = userEvent.setup()
		await open()
		await screen.findByText('Humanizer')
		await user.type(
			screen.getByLabelText('Rechercher une compétence'),
			'naturelle',
		)
		expect(screen.getAllByRole('listitem')).toHaveLength(1)
		await user.clear(screen.getByLabelText('Rechercher une compétence'))
		await user.type(screen.getByLabelText('Rechercher une compétence'), 'zzz')
		expect(screen.getByText('Aucune compétence ne correspond')).toBeTruthy()
	})

	it('shows a load error', async () => {
		await open({
			'GET /api/v1/skills': {
				status: 500,
				body: { code: 'boom', message: 'boom', status: 500 },
			},
		})
		expect(
			await screen.findByText('Impossible de charger les compétences.'),
		).toBeTruthy()
	})

	it('loads the archived skills', async () => {
		const user = userEvent.setup()
		const fetchMock = await open()
		await screen.findByText('Humanizer')
		await user.click(screen.getByRole('button', { name: 'Voir les archivées' }))
		await waitFor(() =>
			expect(
				calls(fetchMock, 'GET').some(
					(c) =>
						c.url.pathname === '/api/v1/skills' &&
						c.url.searchParams.get('archived') === 'true',
				),
			).toBe(true),
		)
	})

	it('opens the detail with content and the agents using it', async () => {
		const user = userEvent.setup()
		await open()
		await user.click(
			await screen.findByRole('button', { name: 'Ouvrir Humanizer' }),
		)
		const dialog = await screen.findByRole('dialog')
		expect(within(dialog).getByText(/Écris simplement/)).toBeTruthy()
		expect(within(dialog).getByText('Utilisée par 2 agents')).toBeTruthy()
		expect(
			within(dialog).getByRole('link', { name: 'Dev' }).getAttribute('href'),
		).toBe('/agents/a-dev/skills')
		expect(within(dialog).queryByText('Autre')).toBeNull()
	})

	it('creates a skill, deriving the slug from the name', async () => {
		const user = userEvent.setup()
		const created = skill('revue-contrat', { name: 'Revue contrat' })
		const fetchMock = await open({
			'POST /api/v1/skills': { status: 201, body: created },
		})
		await screen.findByText('Humanizer')
		await user.click(screen.getByRole('button', { name: 'Nouvelle' }))
		await user.type(screen.getByLabelText('Nom'), 'Revue contrat')
		expect((screen.getByLabelText('Slug') as HTMLInputElement).value).toBe(
			'revue-contrat',
		)
		await user.type(screen.getByLabelText('Contenu (Markdown)'), 'Relis')
		await user.click(screen.getByRole('button', { name: 'Créer' }))
		expect(
			await screen.findByText('Compétence « Revue contrat » créée'),
		).toBeTruthy()
		expect(calls(fetchMock, 'POST')[0]?.body).toEqual({
			slug: 'revue-contrat',
			name: 'Revue contrat',
			description: '',
			content: 'Relis',
			source: 'manual',
		})
	})

	it('validates the slug before sending', async () => {
		const user = userEvent.setup()
		const fetchMock = await open()
		await screen.findByText('Humanizer')
		await user.click(screen.getByRole('button', { name: 'Nouvelle' }))
		await user.type(screen.getByLabelText('Nom'), 'X')
		await user.clear(screen.getByLabelText('Slug'))
		await user.type(screen.getByLabelText('Slug'), 'Bad_Slug')
		await user.click(screen.getByRole('button', { name: 'Créer' }))
		expect(await screen.findByText(/kebab-case/)).toBeTruthy()
		expect(calls(fetchMock, 'POST')).toHaveLength(0)
	})

	it('reports a slug conflict', async () => {
		const user = userEvent.setup()
		await open({
			'POST /api/v1/skills': {
				status: 409,
				body: { code: 'conflict', message: 'exists', status: 409 },
			},
		})
		await screen.findByText('Humanizer')
		await user.click(screen.getByRole('button', { name: 'Nouvelle' }))
		await user.type(screen.getByLabelText('Nom'), 'Humanizer')
		await user.click(screen.getByRole('button', { name: 'Créer' }))
		expect(
			await screen.findByText('Une compétence porte déjà ce slug.'),
		).toBeTruthy()
	})

	it('edits a skill with only the changed fields', async () => {
		const user = userEvent.setup()
		const fetchMock = await open({
			[`PATCH /api/v1/skills/${HUMANIZER.id}`]: {
				...HUMANIZER,
				description: 'Autre',
			},
		})
		await user.click(
			await screen.findByRole('button', { name: 'Ouvrir Humanizer' }),
		)
		await user.click(await screen.findByRole('button', { name: 'Modifier' }))
		const description = screen.getByLabelText('Description')
		await user.clear(description)
		await user.type(description, 'Autre')
		await user.click(screen.getByRole('button', { name: 'Enregistrer' }))
		expect(
			await screen.findByText('Compétence « Humanizer » enregistrée'),
		).toBeTruthy()
		expect(calls(fetchMock, 'PATCH')[0]?.body).toEqual({
			description: 'Autre',
		})
	})

	it('archives a skill', async () => {
		const user = userEvent.setup()
		const fetchMock = await open({
			[`POST /api/v1/skills/${HUMANIZER.id}/archive`]: {
				...HUMANIZER,
				archived: true,
			},
		})
		await user.click(
			await screen.findByRole('button', { name: 'Ouvrir Humanizer' }),
		)
		await user.click(await screen.findByRole('button', { name: 'Archiver' }))
		expect(
			await screen.findByText('Compétence « Humanizer » archivée'),
		).toBeTruthy()
		expect(calls(fetchMock, 'POST')[0]?.url.pathname).toBe(
			`/api/v1/skills/${HUMANIZER.id}/archive`,
		)
	})

	it('restores an archived skill', async () => {
		const user = userEvent.setup()
		const archived = { ...GITLAB, archived: true }
		const fetchMock = await open({
			'GET /api/v1/skills': { items: [archived] },
			[`PATCH /api/v1/skills/${GITLAB.id}`]: GITLAB,
		})
		await user.click(
			await screen.findByRole('button', { name: 'Ouvrir GitLab MR' }),
		)
		expect(screen.getByText('Archivée')).toBeTruthy()
		await user.click(screen.getByRole('button', { name: 'Restaurer' }))
		expect(
			await screen.findByText('Compétence « GitLab MR » restaurée'),
		).toBeTruthy()
		expect(calls(fetchMock, 'PATCH')[0]?.body).toEqual({ archived: false })
	})
})
