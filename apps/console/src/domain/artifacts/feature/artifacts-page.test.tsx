import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { renderAtPath } from '#/components/layout/test-router'
import { ArtifactsPage } from '#/domain/artifacts/feature/artifacts-page'
import { clearAuth } from '#/store/auth.store'
import {
	type MockRoutes,
	mockApi,
	withQueryClient,
} from '#/test-utils/api-mock'

const NOW = new Date().toISOString()
const AGENT_ID = 'a0000000-0000-4000-8000-000000000001'

const artifact = (id: string, extra: Record<string, unknown> = {}) => ({
	id,
	kind: 'markdown',
	title: `Artifact ${id}`,
	created_by: 'agent:iris',
	created_at: NOW,
	updated_at: NOW,
	agent_id: AGENT_ID,
	agent_name: 'Iris (Relais retours)',
	card_id: 'c1',
	card_title: 'Plan de la semaine',
	...extra,
})

const BASE: MockRoutes = {
	'GET /api/v1/team/agents': {
		items: [{ id: AGENT_ID, name: 'Iris (Relais retours)', hue: 120 }],
	},
	'GET /api/v1/artifacts': {
		items: [
			artifact('md'),
			artifact('js', { kind: 'json', title: 'Inventaire' }),
			artifact('ln', {
				kind: 'link',
				title: 'Tableau de bord',
				url: 'https://example.com/board',
			}),
		],
		next_cursor: null,
	},
	'GET /api/v1/artifacts/md': {
		...artifact('md'),
		content: '# Rapport\n\nTout est **vert**.\n\n<img src=x onerror=alert(1)>',
	},
	'GET /api/v1/artifacts/js': {
		...artifact('js', { kind: 'json', title: 'Inventaire' }),
		content: '{"fichiers":3}',
	},
	'GET /api/v1/artifacts/ln': {
		...artifact('ln', {
			kind: 'link',
			title: 'Tableau de bord',
			url: 'https://example.com/board',
		}),
		content: null,
	},
}

afterEach(() => {
	vi.unstubAllGlobals()
	clearAuth()
})

async function open(routes: MockRoutes = BASE) {
	const fetchMock = mockApi(routes)
	await renderAtPath(withQueryClient(<ArtifactsPage />), '/artifacts', [
		'/tasks/$taskId',
	])
	await screen.findByRole('heading', { name: 'Artifacts' })
	return fetchMock
}

describe('ArtifactsPage', () => {
	it('renders the artifacts from the API', async () => {
		await open()
		expect(await screen.findAllByRole('article')).toHaveLength(3)
		expect(screen.getAllByText('Iris')).toHaveLength(3)
		expect(screen.getByText('Données JSON', { selector: 'span' })).toBeTruthy()
		expect(screen.queryByText(/démonstration/i)).toBeNull()
	})

	it('filters by kind and agent through the API', async () => {
		const fetchMock = await open()
		await screen.findAllByRole('article')
		await userEvent.selectOptions(
			screen.getByRole('combobox', { name: 'Filtrer par type' }),
			'json',
		)
		await userEvent.selectOptions(
			screen.getByRole('combobox', { name: 'Filtrer par agent' }),
			AGENT_ID,
		)
		await waitFor(() => {
			const urls = fetchMock.mock.calls
				.map(([input]) => new URL(String(input)))
				.filter((url) => url.pathname === '/api/v1/artifacts')
			const last = urls.at(-1)
			expect(last?.searchParams.get('kind')).toBe('json')
			expect(last?.searchParams.get('agent_id')).toBe(AGENT_ID)
		})
	})

	it('renders a markdown artifact safely', async () => {
		await open()
		await userEvent.click(
			await screen.findByRole('button', { name: /Artifact md/ }),
		)
		const dialog = await screen.findByRole('dialog')
		expect(
			await within(dialog).findByRole('heading', { name: 'Rapport' }),
		).toBeTruthy()
		expect(dialog.querySelector('strong')?.textContent).toBe('vert')
		expect(dialog.querySelector('img')).toBeNull()
		expect(
			within(dialog)
				.getByRole('link', { name: 'Plan de la semaine' })
				.getAttribute('href'),
		).toBe('/tasks/c1')
	})

	it('pretty prints a JSON artifact', async () => {
		await open()
		await userEvent.click(
			await screen.findByRole('button', { name: /Inventaire/ }),
		)
		const dialog = await screen.findByRole('dialog')
		await waitFor(() =>
			expect(dialog.querySelector('pre')?.textContent).toBe(
				'{\n  "fichiers": 3\n}',
			),
		)
	})

	it('opens a link artifact in a new tab', async () => {
		await open()
		await userEvent.click(
			await screen.findByRole('button', { name: /Tableau de bord/ }),
		)
		const dialog = await screen.findByRole('dialog')
		const link = await within(dialog).findByRole('link', {
			name: 'Ouvrir le lien',
		})
		expect(link.getAttribute('href')).toBe('https://example.com/board')
		expect(link.getAttribute('target')).toBe('_blank')
		expect(link.getAttribute('rel')).toContain('noopener')
	})

	it('shows an empty state', async () => {
		await open({
			...BASE,
			'GET /api/v1/artifacts': { items: [], next_cursor: null },
		})
		expect(await screen.findByText('Aucun artifact')).toBeTruthy()
	})
})
