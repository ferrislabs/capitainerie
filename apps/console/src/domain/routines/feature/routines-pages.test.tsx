import { useRouterState } from '@tanstack/react-router'
import { act, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import {
	afterEach,
	beforeEach,
	describe,
	expect,
	it,
	type MockInstance,
	vi,
} from 'vitest'
import { renderAtPath } from '#/components/layout/test-router'
import { resetDemoState } from '#/domain/demo'
import { RoutineDetailPage } from '#/domain/routines/feature/routine-detail-page'
import { RoutineNewPage } from '#/domain/routines/feature/routine-new-page'
import { RoutinesPage } from '#/domain/routines/feature/routines-page'
import {
	resetRoutinesState,
	routinesStore,
} from '#/domain/routines/feature/routines-store'

const EXTRA = [
	'/agents/equipe/$agentId/{-$tab}',
	'/routines',
	'/routines/nouvelle',
	'/routines/$routineId/{-$tab}',
]
const ID = 'analyse-board-hebdomadaire'

let fetchSpy: MockInstance<typeof fetch>

beforeEach(() => {
	resetRoutinesState()
	resetDemoState()
	fetchSpy = vi.spyOn(globalThis, 'fetch')
})

afterEach(() => {
	expect(fetchSpy).not.toHaveBeenCalled()
	fetchSpy.mockRestore()
})

function Where() {
	const path = useRouterState({ select: (s) => s.location.pathname })
	return <output data-testid="where">{path}</output>
}

const detail = (tab?: string) => (
	<>
		<RoutineDetailPage routineId={ID} tab={tab} />
		<Where />
	</>
)

describe('RoutinesPage', () => {
	it('renders the banner, title and cross links', async () => {
		await renderAtPath(<RoutinesPage />, '/routines', EXTRA)
		expect(
			await screen.findByRole('heading', { level: 1, name: 'Routines' }),
		).toBeTruthy()
		expect(screen.getByText(/Données de démonstration/)).toBeTruthy()
		expect(
			screen
				.getByRole('link', { name: 'Toutes les exécutions' })
				.getAttribute('href'),
		).toBe('/runs')
		expect(
			screen
				.getByRole('link', { name: /Créer une routine/ })
				.getAttribute('href'),
		).toBe('/routines/nouvelle')
		expect(screen.getAllByRole('row')).toHaveLength(12)
		expect(screen.getByText('Lun. 06:30')).toBeTruthy()
		expect(screen.getByText('Board Analyst')).toBeTruthy()
	})
	it('launches a routine without navigating', async () => {
		const user = userEvent.setup()
		await renderAtPath(
			<>
				<RoutinesPage />
				<Where />
			</>,
			'/routines',
			EXTRA,
		)
		const row = (
			await screen.findByText('Analyse board (hebdomadaire)')
		).closest('tr')
		if (!row) throw new Error('row')
		await user.click(within(row).getByRole('button', { name: 'Lancer' }))
		expect(within(row).getByText("À l'instant")).toBeTruthy()
		expect(screen.getByText(/Routine lancée/)).toBeTruthy()
		expect(routinesStore.state.routines[0]?.runs).toHaveLength(6)
		expect(screen.getByTestId('where').textContent).toBe('/routines')
	})
	it('toggles the enabled switch', async () => {
		const user = userEvent.setup()
		await renderAtPath(<RoutinesPage />, '/routines', EXTRA)
		const sw = await screen.findByRole('switch', {
			name: 'Activer Analyse board (hebdomadaire)',
		})
		await user.click(sw)
		expect(routinesStore.state.routines[0]?.enabled).toBe(false)
	})
})

describe('RoutineDetailPage', () => {
	it('shows the header and overview by default', async () => {
		await renderAtPath(detail(), `/routines/${ID}`, EXTRA)
		expect(
			await screen.findByRole('heading', {
				level: 1,
				name: 'Analyse board (hebdomadaire)',
			}),
		).toBeTruthy()
		expect(screen.getByText(/Données de démonstration/)).toBeTruthy()
		expect(screen.getByText('Board Analyst · Lun. 06:30 · Active')).toBeTruthy()
		expect(screen.getByText('Prochain passage')).toBeTruthy()
		expect(screen.getByText('Tâches créées')).toBeTruthy()
		expect(screen.getByText('Si déjà ouverte')).toBeTruthy()
		expect(screen.getByText('Mettre en pause')).toBeTruthy()
	})
	it('falls back to the overview for an unknown tab', async () => {
		await renderAtPath(detail('inconnu'), `/routines/${ID}/inconnu`, EXTRA)
		expect(await screen.findByText('Configuration')).toBeTruthy()
	})
	it('exposes one link per tab with aria-current on the active one', async () => {
		await renderAtPath(
			detail('executions'),
			`/routines/${ID}/executions`,
			EXTRA,
		)
		const nav = await screen.findByRole('navigation', {
			name: 'Sections de la routine',
		})
		const links = within(nav).getAllByRole('link')
		expect(links.map((l) => l.getAttribute('href'))).toEqual([
			`/routines/${ID}/apercu`,
			`/routines/${ID}/declencheurs`,
			`/routines/${ID}/executions`,
			`/routines/${ID}/reglages`,
		])
		expect(
			within(nav)
				.getByRole('link', { name: 'Exécutions' })
				.getAttribute('aria-current'),
		).toBe('page')
	})
	it('pauses and launches from the header', async () => {
		const user = userEvent.setup()
		await renderAtPath(detail(), `/routines/${ID}`, EXTRA)
		await user.click(
			await screen.findByRole('button', { name: 'Mettre en pause' }),
		)
		expect(
			screen.getByText('Board Analyst · Lun. 06:30 · En pause'),
		).toBeTruthy()
		expect(screen.getByRole('button', { name: 'Activer' })).toBeTruthy()
		await user.click(screen.getByRole('button', { name: 'Lancer maintenant' }))
		expect(routinesStore.state.routines[0]?.runs).toHaveLength(6)
	})
	it('lists runs and the empty state', async () => {
		await renderAtPath(
			detail('executions'),
			`/routines/${ID}/executions`,
			EXTRA,
		)
		expect(await screen.findAllByText('Planning')).not.toHaveLength(0)
		expect(screen.getAllByRole('row')).toHaveLength(6)
	})
	it('shows the empty state for a never-run routine', async () => {
		await renderAtPath(
			<RoutineDetailPage
				routineId="revue-des-acces-externes-du-drive"
				tab="executions"
			/>,
			'/routines/revue-des-acces-externes-du-drive/executions',
			EXTRA,
		)
		expect(await screen.findByText('Jamais lancée')).toBeTruthy()
	})
	it('edits the schedule and previews next runs', async () => {
		const user = userEvent.setup()
		await renderAtPath(
			detail('declencheurs'),
			`/routines/${ID}/declencheurs`,
			EXTRA,
		)
		const time = await screen.findByLabelText('Heure')
		await user.clear(time)
		await user.type(time, '07:15')
		expect(screen.getByText('Lun. 07:15')).toBeTruthy()
		await user.click(screen.getByRole('checkbox', { name: 'Mer.' }))
		expect(screen.getByText('Lun., Mer. 07:15')).toBeTruthy()
		await user.selectOptions(
			screen.getByLabelText('Fréquence'),
			'Expression cron',
		)
		expect(screen.getByLabelText('Expression cron')).toBeTruthy()
		expect(screen.getByText(/calculés par le serveur/)).toBeTruthy()
		expect(screen.getByText(/Lancement manuel/)).toBeTruthy()
	})
	it('adds and manages a webhook trigger', async () => {
		const user = userEvent.setup()
		await renderAtPath(
			detail('declencheurs'),
			`/routines/${ID}/declencheurs`,
			EXTRA,
		)
		await user.selectOptions(
			await screen.findByLabelText('Type de déclencheur'),
			'Webhook',
		)
		await user.click(
			screen.getByRole('button', { name: 'Ajouter un déclencheur' }),
		)
		expect(screen.getByRole('button', { name: 'Copier' })).toBeTruthy()
		expect(screen.getByText(/whsec_••••••••/)).toBeTruthy()
		await user.click(screen.getByRole('button', { name: 'Régénérer' }))
		expect(screen.getByText(/Nouveau secret généré/)).toBeTruthy()
		expect(
			screen.getByRole('switch', { name: 'Activer le webhook' }),
		).toBeTruthy()
		await user.click(
			screen.getAllByRole('button', { name: 'Retirer' })[1] as HTMLElement,
		)
		expect(screen.queryByRole('button', { name: 'Copier' })).toBeNull()
	})
	it('validates and saves the settings', async () => {
		const user = userEvent.setup()
		await renderAtPath(detail('reglages'), `/routines/${ID}/reglages`, EXTRA)
		const name = await screen.findByLabelText(/Nom/)
		await user.clear(name)
		await user.click(screen.getByRole('button', { name: 'Enregistrer' }))
		expect(screen.getByText('Le nom est obligatoire')).toBeTruthy()
		await user.type(name, 'Bilan')
		await user.click(screen.getByRole('button', { name: 'Enregistrer' }))
		expect(routinesStore.state.routines[0]?.name).toBe('Bilan')
	})
	it('shows connectors of the chosen agent or a warning', async () => {
		const user = userEvent.setup()
		await renderAtPath(detail('reglages'), `/routines/${ID}/reglages`, EXTRA)
		await screen.findByLabelText(/Nom/)
		expect(
			screen.getByText("Cet agent n'a accès à aucun connecteur"),
		).toBeTruthy()
		await user.selectOptions(
			screen.getByLabelText('Agent'),
			'Archibald (Archivage comptable)',
		)
		expect(
			screen.getByText(/Connecteurs autorisés pour cet agent/),
		).toBeTruthy()
	})
	it('cancels edits', async () => {
		const user = userEvent.setup()
		await renderAtPath(detail('reglages'), `/routines/${ID}/reglages`, EXTRA)
		const name = await screen.findByLabelText(/Nom/)
		await user.type(name, ' x')
		await user.click(screen.getByRole('button', { name: 'Annuler' }))
		expect((screen.getByLabelText(/Nom/) as HTMLInputElement).value).toBe(
			'Analyse board (hebdomadaire)',
		)
	})
	it('reports an unknown routine', async () => {
		await renderAtPath(
			<RoutineDetailPage routineId="nope" />,
			'/routines/nope',
			EXTRA,
		)
		expect(await screen.findByText('Routine introuvable')).toBeTruthy()
	})
})

describe('RoutineNewPage', () => {
	it('refuses a missing name', async () => {
		const user = userEvent.setup()
		await renderAtPath(<RoutineNewPage />, '/routines/nouvelle', EXTRA)
		await user.click(
			await screen.findByRole('button', { name: 'Créer la routine' }),
		)
		expect(screen.getByText('Donne un nom à la routine')).toBeTruthy()
		expect(routinesStore.state.routines).toHaveLength(11)
	})
	it('requires a weekday for weekly schedules', async () => {
		const user = userEvent.setup()
		await renderAtPath(<RoutineNewPage />, '/routines/nouvelle', EXTRA)
		await user.type(await screen.findByLabelText(/Nom/), 'Test')
		await user.click(screen.getByRole('checkbox', { name: 'Lun.' }))
		await user.click(screen.getByRole('button', { name: 'Créer la routine' }))
		expect(screen.getByText('Choisis au moins un jour')).toBeTruthy()
	})
	it('creates the routine, navigates to it and lists it', async () => {
		const user = userEvent.setup()
		await renderAtPath(
			<>
				<RoutineNewPage />
				<Where />
			</>,
			'/routines/nouvelle',
			EXTRA,
		)
		await user.type(await screen.findByLabelText(/Nom/), 'Bilan été')
		await user.click(screen.getByRole('checkbox', { name: 'Mer.' }))
		expect(screen.getByText('Prochains passages')).toBeTruthy()
		await user.click(screen.getByRole('button', { name: 'Créer la routine' }))
		await waitFor(() =>
			expect(screen.getByTestId('where').textContent).toBe(
				'/routines/bilan-ete',
			),
		)
		expect(routinesStore.state.routines).toHaveLength(12)
	})
	it('appears in the list with its schedule text', async () => {
		const user = userEvent.setup()
		await renderAtPath(<RoutineNewPage />, '/routines/nouvelle', EXTRA)
		await user.type(await screen.findByLabelText(/Nom/), 'Bilan été')
		await user.click(screen.getByRole('button', { name: 'Créer la routine' }))
		await act(async () => undefined)
		const list = await renderAtPath(<RoutinesPage />, '/routines', EXTRA)
		expect(await within(list.container).findByText('Bilan été')).toBeTruthy()
		expect(
			within(list.container).getAllByText('Lun. 08:00').length,
		).toBeGreaterThan(0)
	})
})
