import { fireEvent, screen, within } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { renderAtPath } from '#/components/layout/test-router'
import {
	demoStore,
	grantConnector,
	resetDemoState,
	revokeConnector,
} from '#/domain/demo'
import { SourceDetailPage } from '#/domain/sources/feature/source-detail-page'
import { resetPipelines } from '#/domain/sources/mock/pipeline-store'

const ID = 'nouveaux-sous-domaines-sensibles'
const ROUTE = [
	'/sources/$sourceId/{-$tab}',
	'/agents/$agentId/{-$tab}',
	'/tasks',
	'/tasks/$taskId',
]
const src = () => demoStore.state.sources.find((s) => s.id === ID)

async function open(tab?: string) {
	const path = tab ? `/sources/${ID}/${tab}` : `/sources/${ID}`
	await renderAtPath(<SourceDetailPage sourceId={ID} tab={tab} />, path, ROUTE)
}

describe('SourceDetailPage', () => {
	beforeEach(() => {
		resetDemoState()
		resetPipelines()
	})

	it('renders header, banner and five tab links', async () => {
		const fetchSpy = vi.spyOn(globalThis, 'fetch')
		await open()
		expect(
			await screen.findByRole('heading', {
				name: 'Nouveaux sous-domaines sensibles',
			}),
		).toBeTruthy()
		expect(screen.getByText(/démonstration/i)).toBeTruthy()
		expect(screen.getByText('Active')).toBeTruthy()
		const nav = screen.getByRole('navigation', {
			name: 'Sections de la source',
		})
		const links = within(nav).getAllByRole('link')
		expect(links.map((l) => l.textContent)).toEqual([
			"Vue d'ensemble",
			'Motifs et exclusions',
			'Pipeline',
			'Événements',
			'Agents',
		])
		expect(links[0]?.getAttribute('aria-current')).toBe('page')
		expect(links[2]?.getAttribute('href')).toBe(`/sources/${ID}/pipeline`)
		expect(fetchSpy).not.toHaveBeenCalled()
	})

	it('marks the active tab from the URL and falls back to overview', async () => {
		await open('regles')
		const nav = await screen.findByRole('navigation', {
			name: 'Sections de la source',
		})
		expect(
			within(nav)
				.getByRole('link', { name: 'Motifs et exclusions' })
				.getAttribute('aria-current'),
		).toBe('page')
	})

	it('unknown tab shows the overview', async () => {
		await open('nimporte')
		expect(await screen.findByText('Événements 24 h')).toBeTruthy()
		expect(screen.getByText('Configuration')).toBeTruthy()
	})

	it('overview shows tiles and the flow', async () => {
		await open('apercu')
		expect(await screen.findByText('Tâches créées')).toBeTruthy()
		expect(screen.getByText('Doublons')).toBeTruthy()
		expect(screen.getByText('3. Agent')).toBeTruthy()
	})

	it('pauses and resumes, and sends a test', async () => {
		await open()
		fireEvent.click(
			await screen.findByRole('button', { name: 'Mettre en pause' }),
		)
		expect(src()?.enabled).toBe(false)
		expect(await screen.findByText('En pause')).toBeTruthy()
		fireEvent.click(screen.getByRole('button', { name: 'Activer' }))
		expect(src()?.enabled).toBe(true)
		fireEvent.click(screen.getByRole('button', { name: 'Envoyer un test' }))
		expect(
			await screen.findByText('Événement de test envoyé : iam.*'),
		).toBeTruthy()
	})

	it('shows a not found state', async () => {
		await renderAtPath(
			<SourceDetailPage sourceId="zzz" />,
			'/sources/zzz',
			ROUTE,
		)
		expect(await screen.findByText('Source introuvable')).toBeTruthy()
	})

	describe('rules tab', () => {
		it('adds and removes patterns and exclusions', async () => {
			await open('regles')
			fireEvent.change(await screen.findByLabelText('Nouveau motif'), {
				target: { value: 'login.*' },
			})
			fireEvent.click(
				screen.getAllByRole('button', { name: 'Ajouter' })[0] as HTMLElement,
			)
			expect(src()?.patterns).toEqual(['iam.*', 'auth.*', 'login.*'])
			fireEvent.click(screen.getByRole('button', { name: 'Retirer iam.*' }))
			expect(src()?.patterns).toEqual(['auth.*', 'login.*'])
			fireEvent.change(screen.getByLabelText('Nouvelle exclusion'), {
				target: { value: '*.test.fr' },
			})
			fireEvent.click(
				screen.getAllByRole('button', { name: 'Ajouter' })[1] as HTMLElement,
			)
			expect(src()?.exclusions).toContain('*.test.fr')
		})

		it('tests a value live', async () => {
			await open('regles')
			const field = await screen.findByLabelText('Valeur à tester')
			fireEvent.change(field, { target: { value: 'iam.acme.fr' } })
			expect(screen.getByText('iam.*', { selector: 'b' })).toBeTruthy()
			fireEvent.change(field, { target: { value: 'auth.getnatalia.com' } })
			expect(screen.getByText(/Ignoré : exclu par/)).toBeTruthy()
			fireEvent.change(field, { target: { value: 'zzz.fr' } })
			expect(screen.getByText(/Aucun motif ne correspond/)).toBeTruthy()
		})
	})

	it('events tab lists decisions with linked task ids', async () => {
		await open('evenements')
		expect(
			(await screen.findAllByText('iam.acme-logistique.fr', { selector: 'td' }))
				.length,
		).toBe(2)
		expect(screen.getAllByText('Doublon').length).toBeGreaterThan(0)
		expect(screen.getAllByText('Sans correspondance')).toHaveLength(2)
		expect(
			screen.getByRole('link', { name: 'NAT-690' }).getAttribute('href'),
		).toBe('/tasks/nat-690')
		expect(screen.getByText('Déjà traité : NAT-690')).toBeTruthy()
	})

	describe('agents tab', () => {
		it('detaches and attaches an agent', async () => {
			await open('agents')
			fireEvent.click(
				await screen.findByRole('button', { name: 'Retirer Veille légale' }),
			)
			expect(src()?.agentIds).toEqual([])
			expect(screen.getByText(/Aucun agent n'écoute/)).toBeTruthy()
			fireEvent.change(screen.getByLabelText('Agent'), {
				target: { value: 'archibald' },
			})
			fireEvent.click(screen.getByRole('button', { name: 'Ajouter' }))
			expect(src()?.agentIds).toEqual(['archibald'])
		})

		it('saves the task template', async () => {
			await open('agents')
			fireEvent.change(await screen.findByLabelText('Modèle de tâche'), {
				target: { value: 'Nouveau {hôte}' },
			})
			fireEvent.click(screen.getByRole('button', { name: 'Enregistrer' }))
			expect(src()?.template).toBe('Nouveau {hôte}')
			expect(await screen.findByText('Modèle enregistré')).toBeTruthy()
		})
	})

	describe('pipeline tab', () => {
		it('shows the flow, dossier, verdict and criteria', async () => {
			await open('pipeline')
			expect(await screen.findByText('2. Recherche (IA)')).toBeTruthy()
			expect(screen.getByText(/délègue à/)).toBeTruthy()
			expect(screen.getByText('ACME LOGISTIQUE SAS')).toBeTruthy()
			expect(screen.getByText('Qualifié')).toBeTruthy()
			fireEvent.click(screen.getByRole('button', { name: 'lumen' }))
			expect(screen.getByText('LUMEN SANTE SARL')).toBeTruthy()
			expect(screen.getByText('Rejeté')).toBeTruthy()
			expect(screen.getByText('Aucun export pour ce dossier')).toBeTruthy()
		})

		it('reacts to criteria edits and presets', async () => {
			await open('pipeline')
			await screen.findByText('Qualifié')
			fireEvent.click(screen.getByRole('button', { name: 'novabank' }))
			expect(
				screen.getByText('À valider', { selector: 'span.inline-flex' }),
			).toBeTruthy()
			fireEvent.change(screen.getByLabelText('Si non : Effectif minimum'), {
				target: { value: 'ignore' },
			})
			fireEvent.change(
				screen.getByLabelText(
					'Si non : SIREN trouvé et confirmé par 2 indices',
				),
				{
					target: { value: 'ignore' },
				},
			)
			expect(screen.getByText('Qualifié')).toBeTruthy()
			fireEvent.click(screen.getByRole('button', { name: 'Minimal' }))
			expect(screen.getAllByRole('switch', { name: /^Activer / })).toHaveLength(
				2,
			)
			fireEvent.change(screen.getByLabelText('Critère à ajouter'), {
				target: { value: 'age' },
			})
			fireEvent.click(
				screen.getByRole('button', { name: 'Ajouter un critère' }),
			)
			expect(screen.getAllByRole('switch', { name: /^Activer / })).toHaveLength(
				3,
			)
			fireEvent.click(
				screen.getByRole('button', {
					name: 'Retirer Entreprise active (non radiée)',
				}),
			)
			expect(screen.getAllByRole('switch', { name: /^Activer / })).toHaveLength(
				2,
			)
		})

		it('shows the dry-run payload then blocks without CRM access', async () => {
			await open('pipeline')
			await screen.findByText('Qualifié')
			expect(screen.getByText('Charge envoyée au CRM')).toBeTruthy()
			expect(document.querySelector('pre')?.textContent).toContain(
				'"mode": "dry_run"',
			)
			fireEvent.click(screen.getByRole('switch', { name: 'Essai à blanc' }))
			expect(document.querySelector('pre')?.textContent).toContain(
				'"mode": "write"',
			)
			revokeConnector('veille-legale', 'Odoo CRM (MCP)')
			expect(
				await screen.findByText(
					"Export bloqué : aucun agent n'a accès au connecteur Odoo CRM (MCP)",
				),
			).toBeTruthy()
		})

		it('grants a connector from the access table', async () => {
			revokeConnector('veille-legale', 'BODACC')
			revokeConnector('registre', 'BODACC')
			await open('pipeline')
			fireEvent.click(
				await screen.findByRole('button', { name: 'Autoriser Veille légale' }),
			)
			expect(demoStore.state.grants['veille-legale']).toContain('BODACC')
			expect(
				screen.queryByRole('button', { name: 'Autoriser Veille légale' }),
			).toBeNull()
		})

		it('flags uninstalled CRM destinations', async () => {
			grantConnector('veille-legale', 'Odoo CRM (MCP)')
			await open('pipeline')
			fireEvent.change(await screen.findByLabelText('Destination'), {
				target: { value: 'HubSpot (MCP)' },
			})
			expect(screen.getByText('Connecteur non installé')).toBeTruthy()
			expect(screen.getByText(/Export bloqué/)).toBeTruthy()
		})
	})
})

describe('pipeline validation request', () => {
	beforeEach(() => {
		resetDemoState()
		resetPipelines()
	})

	it('only offers the request when the verdict is to validate', async () => {
		await open('pipeline')
		await screen.findByText('Qualifié')
		expect(
			screen.queryByRole('button', { name: 'Demander une validation' }),
		).toBeNull()
	})

	it('creates one pending approval per dossier and source', async () => {
		await open('pipeline')
		await screen.findByText('Qualifié')
		fireEvent.click(screen.getByRole('button', { name: 'novabank' }))
		expect(screen.queryByText('Voir les approbations')).toBeNull()
		const before = demoStore.state.approvals.length
		fireEvent.click(
			screen.getByRole('button', { name: 'Demander une validation' }),
		)
		expect(demoStore.state.approvals).toHaveLength(before + 1)
		const created = demoStore.state.approvals[0]
		expect(created?.id).toBe(`APP-${before + 1}`)
		expect(created?.status).toBe('pending')
		expect(created?.title).toBe(
			'Valider le rapprochement : NOVABANK GROUPE (Nouveaux sous-domaines sensibles)',
		)
		expect(created?.requestedBy).toBe(src()?.agentIds[0])
		expect(created?.detail).not.toBe('')
		expect(screen.getByText('Voir les approbations').getAttribute('href')).toBe(
			'/approvals',
		)
		fireEvent.click(
			screen.getByRole('button', { name: 'Demander une validation' }),
		)
		expect(demoStore.state.approvals).toHaveLength(before + 1)
		expect(screen.getByText('Demande déjà envoyée')).toBeTruthy()
	})
})
