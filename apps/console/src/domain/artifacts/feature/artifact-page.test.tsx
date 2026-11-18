import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, onTestFinished, vi } from 'vitest'
import { renderAtPath } from '#/components/layout/test-router'
import { ArtifactPage } from '#/domain/artifacts/feature/artifact-page'
import { clearAuth } from '#/store/auth.store'
import {
	type MockRoutes,
	mockApi,
	withQueryClient,
} from '#/test-utils/api-mock'

const NOW = new Date().toISOString()

const REPORT = [
	'# Audit de sécurité',
	'',
	'## Résumé',
	'',
	'Deux constats, dont un **Critique**.',
	'',
	'## Constats',
	'',
	'| Sévérité | Fichier | Constat |',
	'|---|---|---|',
	'| Critique | `crates/infra/src/jwks.rs:420` | Clé non vérifiée |',
	'| Basse | `apps/console/src/a.ts:12` | Message \\| trompeur |',
	'',
	'### Détail',
	'',
	'- point',
	'  - sous-point',
].join('\n')

const ARTIFACT = {
	id: 'audit',
	kind: 'markdown',
	title: 'Audit sécurité JWKS',
	created_by: 'agent:vigie',
	created_at: NOW,
	updated_at: NOW,
	agent_id: null,
	agent_name: 'Vigie (Sécurité)',
	card_id: 'c1',
	card_title: 'Auditer le dépôt',
	content: REPORT,
}

const BASE: MockRoutes = {
	'GET /api/v1/team/agents': { items: [] },
	'GET /api/v1/artifacts/audit': ARTIFACT,
}

afterEach(() => {
	vi.unstubAllGlobals()
	vi.restoreAllMocks()
	clearAuth()
})

async function open(routes: MockRoutes = BASE, id = 'audit') {
	mockApi(routes)
	await renderAtPath(
		withQueryClient(<ArtifactPage artifactId={id} />),
		`/artifacts/${id}`,
	)
}

describe('ArtifactPage', () => {
	it('renders the report with its table and table of contents', async () => {
		await open()
		expect(
			await screen.findByRole('heading', {
				level: 1,
				name: 'Audit sécurité JWKS',
			}),
		).toBeTruthy()
		const table = screen.getByRole('table')
		expect(within(table).getAllByRole('row')).toHaveLength(3)
		expect(within(table).getByText('crates/infra/src/jwks.rs:420')).toBeTruthy()
		expect(within(table).getByText('Message | trompeur')).toBeTruthy()
		expect(
			table.querySelector('[data-severity="critical"]')?.textContent,
		).toContain('Critique')
		expect(table.querySelector('[data-severity="low"]')).toBeTruthy()

		const toc = screen.getByRole('navigation', { name: 'Sommaire' })
		const links = within(toc).getAllByRole('link')
		expect(links.map((link) => link.textContent)).toEqual([
			'Audit de sécurité',
			'Résumé',
			'Constats',
			'Détail',
		])
		expect(links[2]?.getAttribute('href')).toBe('#constats')
		expect(document.getElementById('constats')?.textContent).toBe('Constats')

		expect(
			screen
				.getByRole('link', { name: 'Auditer le dépôt' })
				.getAttribute('href'),
		).toBe('/tasks/c1')
		expect(
			screen
				.getByRole('link', { name: 'Ouvrir la tâche' })
				.getAttribute('href'),
		).toBe('/tasks/c1')
	})

	it('copies the Markdown source', async () => {
		const user = userEvent.setup()
		await open()
		const writeText = vi
			.spyOn(navigator.clipboard, 'writeText')
			.mockResolvedValue(undefined)
		await user.click(
			await screen.findByRole('button', { name: 'Copier le Markdown' }),
		)
		expect(writeText).toHaveBeenCalledWith(REPORT)
		expect(await screen.findByRole('button', { name: 'Copié' })).toBeTruthy()
	})

	it('downloads the Markdown as a file named after the title', async () => {
		const createObjectURL = vi.fn((_blob: Blob) => 'blob:report')
		const revokeObjectURL = vi.fn()
		const original = {
			create: URL.createObjectURL,
			revoke: URL.revokeObjectURL,
		}
		URL.createObjectURL = createObjectURL
		URL.revokeObjectURL = revokeObjectURL
		onTestFinished(() => {
			URL.createObjectURL = original.create
			URL.revokeObjectURL = original.revoke
		})
		const click = vi
			.spyOn(HTMLAnchorElement.prototype, 'click')
			.mockImplementation(function (this: HTMLAnchorElement) {
				expect(this.download).toBe('audit-securite-jwks.md')
				expect(this.href).toBe('blob:report')
			})
		await open()
		await userEvent.click(
			await screen.findByRole('button', { name: 'Télécharger (.md)' }),
		)
		expect(click).toHaveBeenCalledOnce()
		const blob = createObjectURL.mock.calls[0]?.[0]
		expect(blob?.type).toBe('text/markdown;charset=utf-8')
		expect(await blob?.text()).toBe(REPORT)
		await waitFor(() =>
			expect(revokeObjectURL).toHaveBeenCalledWith('blob:report'),
		)
	})

	it('pretty prints a JSON artifact without a table of contents', async () => {
		await open(
			{
				...BASE,
				'GET /api/v1/artifacts/inv': {
					...ARTIFACT,
					id: 'inv',
					kind: 'json',
					title: 'Inventaire',
					content: '{"fichiers":3}',
				},
			},
			'inv',
		)
		await screen.findByRole('heading', { level: 1, name: 'Inventaire' })
		expect(document.querySelector('pre')?.textContent).toBe(
			'{\n  "fichiers": 3\n}',
		)
		expect(screen.queryByRole('navigation', { name: 'Sommaire' })).toBeNull()
		expect(
			screen.getByRole('button', { name: 'Télécharger (.json)' }),
		).toBeTruthy()
	})

	it('reports a missing artifact', async () => {
		await open(BASE, 'gone')
		expect(
			await screen.findByText(/Impossible de charger cet artifact/),
		).toBeTruthy()
	})
})
