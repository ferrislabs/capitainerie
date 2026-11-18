import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { renderAtPath } from '#/components/layout/test-router'
import { SourceNewPage } from '#/domain/sources/feature/source-new-page'
import { source } from '#/domain/sources/test-fixtures'
import { clearAuth } from '#/store/auth.store'
import { mockApi, withQueryClient } from '#/test-utils/api-mock'
import { DEV } from '#/test-utils/team-agents'

const EXTRA = ['/sources/nouvelle', '/sources/$sourceId/{-$tab}']
const URL_ONCE = 'https://api.test/hooks/sources/tok_secret'

type Routes = Parameters<typeof mockApi>[0]

async function open(routes: Routes = {}) {
	const fetchMock = mockApi({
		'GET /api/v1/team/agents': { items: [DEV] },
		'GET /api/v1/projects': {
			items: [
				{
					open_cards: 0,
					project: {
						id: 'p1',
						name: 'Compta',
						aliases: [],
						archived: false,
						created_at: '',
						created_by: 'u',
						description: '',
						updated_at: '',
					},
				},
			],
		},
		'POST /api/v1/sources': {
			status: 201,
			body: {
				source: source('s-new', { name: 'Pièces Chatwoot' }),
				token: 'tok_secret',
				webhook_url: URL_ONCE,
			},
		},
		...routes,
	})
	await renderAtPath(
		withQueryClient(<SourceNewPage />),
		'/sources/nouvelle',
		EXTRA,
	)
	await screen.findByRole('option', { name: DEV.name })
	return fetchMock
}

function posted(fetchMock: ReturnType<typeof mockApi>) {
	const call = fetchMock.mock.calls.find(([, init]) => init?.method === 'POST')
	return call ? JSON.parse(String(call[1]?.body)) : undefined
}

afterEach(() => {
	vi.unstubAllGlobals()
	clearAuth()
})

describe('SourceNewPage', () => {
	it('has no demo-only fields', async () => {
		await open()
		expect(
			screen.getByRole('heading', { name: 'Nouvelle source' }),
		).toBeTruthy()
		expect(screen.queryByLabelText('Exclusions')).toBeNull()
		expect(screen.queryByLabelText('Dédoublonnage')).toBeNull()
		expect(screen.queryByText(/démonstration/i)).toBeNull()
		expect(screen.getByText(/sans demander d'approbation/)).toBeTruthy()
	})

	it('derives the slug from the name until it is edited', async () => {
		const user = userEvent.setup()
		await open()
		await user.type(screen.getByLabelText('Nom'), 'Pièces reçues')
		expect((screen.getByLabelText('Slug') as HTMLInputElement).value).toBe(
			'pieces-recues',
		)
		await user.clear(screen.getByLabelText('Slug'))
		await user.type(screen.getByLabelText('Slug'), 'perso')
		await user.type(screen.getByLabelText('Nom'), ' X')
		expect((screen.getByLabelText('Slug') as HTMLInputElement).value).toBe(
			'perso',
		)
	})

	it('refuses an empty name without calling the API', async () => {
		const user = userEvent.setup()
		const fetchMock = await open()
		await user.click(screen.getByRole('button', { name: 'Créer la source' }))
		expect(screen.getByRole('alert').textContent).toMatch(/nom/)
		expect(posted(fetchMock)).toBeUndefined()
	})

	it('creates the source and shows its URL once', async () => {
		const user = userEvent.setup()
		const fetchMock = await open()
		await user.type(screen.getByLabelText('Nom'), 'Pièces Chatwoot')
		await user.selectOptions(screen.getByLabelText('Agent par défaut'), DEV.id)
		await user.selectOptions(screen.getByLabelText('Projet'), 'p1')
		await user.click(
			screen.getByRole('switch', { name: "Lancer l'agent automatiquement" }),
		)
		await user.click(
			screen.getByRole('switch', { name: 'Écritures sans approbation' }),
		)
		await user.click(screen.getByRole('button', { name: 'Créer la source' }))
		expect(await screen.findByText(/ne sera plus affichée/)).toBeTruthy()
		expect(screen.getByLabelText('URL du webhook').textContent).toBe(URL_ONCE)
		expect(screen.getByLabelText('Token').textContent).toBe('tok_secret')
		expect(
			screen.getByRole('button', { name: 'Ouvrir la source' }),
		).toBeTruthy()
		await waitFor(() =>
			expect(posted(fetchMock)).toEqual({
				kind: 'webhook',
				name: 'Pièces Chatwoot',
				slug: 'pieces-chatwoot',
				description: '',
				enabled: true,
				default_agent_id: DEV.id,
				project_id: 'p1',
				auto_run: true,
				allow_writes: true,
			}),
		)
	})

	it('explains a slug conflict', async () => {
		const user = userEvent.setup()
		await open({
			'POST /api/v1/sources': {
				status: 409,
				body: { code: 'conflict', message: 'slug', status: 409 },
			},
		})
		await user.type(screen.getByLabelText('Nom'), 'Doublon')
		await user.click(screen.getByRole('button', { name: 'Créer la source' }))
		expect(
			await screen.findByText('Une source porte déjà ce slug.'),
		).toBeTruthy()
	})
})
