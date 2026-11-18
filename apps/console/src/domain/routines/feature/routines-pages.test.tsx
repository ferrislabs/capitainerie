import { useRouterState } from '@tanstack/react-router'
import { fireEvent, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { renderAtPath } from '#/components/layout/test-router'
import { RoutineDetailPage } from '#/domain/routines/feature/routine-detail-page'
import { RoutineNewPage } from '#/domain/routines/feature/routine-new-page'
import { RoutinesPage } from '#/domain/routines/feature/routines-page'
import { routine, routineRun, trigger } from '#/domain/routines/test-fixtures'
import { clearAuth } from '#/store/auth.store'
import { mockApi, withQueryClient } from '#/test-utils/api-mock'
import { DEV, TEAM } from '#/test-utils/team-agents'

const EXTRA = [
	'/routines',
	'/routines/nouvelle',
	'/routines/$routineId/{-$tab}',
]
const ID = 'r1'
const CARD = '6f1c2a90-1111-4000-8000-000000000001'
const PLANNING = trigger('t1')
const WEBHOOK = trigger('t2', {
	kind: 'webhook',
	schedule: null,
	next_run_at: null,
	token_hint: 'k9Qz',
})
const LAST = routineRun('run-1', {
	status: 'created',
	card_id: CARD,
	card_column: 'done',
	run_started_at: '2026-11-17T06:00:00Z',
	run_finished_at: '2026-11-17T06:01:12Z',
})
const ROUTINE = routine(ID, {
	name: 'Analyse board (hebdomadaire)',
	slug: 'analyse-board',
	agent_id: DEV.id,
	prompt: 'Analyser le board et documenter.',
	next_run_at: '2026-11-23T05:30:00Z',
	last_run_at: '2026-11-17T06:00:01Z',
	last_run: LAST,
	counts: { created: 4, skipped: 1, failed: 2 },
	triggers: [PLANNING],
})

type Routes = Parameters<typeof mockApi>[0]

const BASE: Routes = {
	'GET /api/v1/team/agents': { items: TEAM },
	'GET /api/v1/projects': { items: [] },
}

function calls(fetchMock: ReturnType<typeof mockApi>, method: string) {
	return fetchMock.mock.calls
		.filter(([, init]) => (init?.method ?? 'GET') === method)
		.map(([url, init]) => ({
			url: new URL(String(url)),
			body: init?.body ? JSON.parse(String(init.body)) : undefined,
		}))
}

function Where() {
	const path = useRouterState({ select: (s) => s.location.pathname })
	return <output data-testid="where">{path}</output>
}

afterEach(() => {
	vi.unstubAllGlobals()
	clearAuth()
})

describe('RoutinesPage', () => {
	async function open(routes: Routes = {}) {
		const fetchMock = mockApi({
			...BASE,
			'GET /api/v1/routines': {
				items: [
					ROUTINE,
					routine('r2', {
						name: 'Newsletter produit',
						enabled: false,
						agent_id: 'a-legal',
					}),
				],
			},
			...routes,
		})
		await renderAtPath(withQueryClient(<RoutinesPage />), '/routines', EXTRA)
		await screen.findByText('Analyse board (hebdomadaire)')
		return fetchMock
	}

	it('lists the routines with schedule, last run and counts', async () => {
		await open()
		expect(
			screen.getByRole('heading', { level: 1, name: 'Routines' }),
		).toBeTruthy()
		expect(screen.queryByText(/Données de démonstration/)).toBeNull()
		expect(
			screen
				.getByRole('link', { name: /Créer une routine/ })
				.getAttribute('href'),
		).toBe('/routines/nouvelle')
		const row = screen
			.getByText('Analyse board (hebdomadaire)')
			.closest('tr') as HTMLElement
		expect(within(row).getByText('Lun 06:30')).toBeTruthy()
		expect(within(row).getByText('terminée')).toBeTruthy()
		expect(within(row).getByText(DEV.name)).toBeTruthy()
		expect(
			within(row).getByTitle(/4 tâches créées, 1 ignorés, 2 échecs/),
		).toBeTruthy()
		const paused = screen
			.getByText('Newsletter produit')
			.closest('tr') as HTMLElement
		expect(within(paused).getByText('En pause')).toBeTruthy()
		expect(within(paused).getByText('Jamais')).toBeTruthy()
	})

	it('filters with the search box', async () => {
		const user = userEvent.setup()
		await open()
		await user.type(
			screen.getByRole('searchbox', { name: 'Rechercher une routine' }),
			'news',
		)
		expect(screen.queryByText('Analyse board (hebdomadaire)')).toBeNull()
		expect(screen.getByText('Newsletter produit')).toBeTruthy()
		await user.type(
			screen.getByRole('searchbox', { name: 'Rechercher une routine' }),
			'zzz',
		)
		expect(screen.getByText('Aucune routine ne correspond')).toBeTruthy()
	})

	it('asks the archived routines', async () => {
		const user = userEvent.setup()
		const fetchMock = await open()
		await user.click(screen.getByRole('button', { name: 'Voir les archivées' }))
		await waitFor(() =>
			expect(
				calls(fetchMock, 'GET').some(
					({ url }) =>
						url.pathname === '/api/v1/routines' &&
						url.searchParams.get('archived') === 'true',
				),
			).toBe(true),
		)
	})

	it('toggles a routine with a PATCH', async () => {
		const user = userEvent.setup()
		const fetchMock = await open({
			[`PATCH /api/v1/routines/${ID}`]: { ...ROUTINE, enabled: false },
		})
		await user.click(
			screen.getByRole('switch', {
				name: 'Activer Analyse board (hebdomadaire)',
			}),
		)
		await waitFor(() =>
			expect(calls(fetchMock, 'PATCH')[0]?.body).toEqual({ enabled: false }),
		)
		expect(
			await screen.findByText(/Routine en pause : Analyse board/),
		).toBeTruthy()
	})

	it('launches a routine without navigating', async () => {
		const user = userEvent.setup()
		const fetchMock = mockApi({
			...BASE,
			'GET /api/v1/routines': { items: [ROUTINE] },
			[`POST /api/v1/routines/${ID}/launch`]: {
				status: 202,
				body: routineRun('run-2', { status: 'pending', origin: 'manual' }),
			},
		})
		await renderAtPath(
			withQueryClient(
				<>
					<RoutinesPage />
					<Where />
				</>,
			),
			'/routines',
			EXTRA,
		)
		await user.click(
			await screen.findByRole('button', {
				name: 'Lancer Analyse board (hebdomadaire)',
			}),
		)
		await waitFor(() =>
			expect(calls(fetchMock, 'POST')[0]?.url.pathname).toBe(
				`/api/v1/routines/${ID}/launch`,
			),
		)
		expect(await screen.findByText(/Routine lancée/)).toBeTruthy()
		expect(screen.getByTestId('where').textContent).toBe('/routines')
	})

	it('shows the empty and error states', async () => {
		mockApi({ ...BASE, 'GET /api/v1/routines': { items: [] } })
		await renderAtPath(withQueryClient(<RoutinesPage />), '/routines', EXTRA)
		expect(await screen.findByText('Aucune routine')).toBeTruthy()
	})

	it('reports a loading failure', async () => {
		mockApi({
			...BASE,
			'GET /api/v1/routines': {
				status: 500,
				body: { code: 'x', message: 'x' },
			},
		})
		await renderAtPath(withQueryClient(<RoutinesPage />), '/routines', EXTRA)
		expect(
			await screen.findByText('Impossible de charger les routines.'),
		).toBeTruthy()
	})
})

describe('RoutineDetailPage', () => {
	async function open(tab?: string, routes: Routes = {}, data = ROUTINE) {
		const fetchMock = mockApi({
			...BASE,
			[`GET /api/v1/routines/${ID}`]: data,
			[`GET /api/v1/routines/${ID}/runs`]: {
				items: [
					LAST,
					routineRun('run-0', {
						status: 'skipped',
						reason: 'La tâche précédente est encore ouverte',
					}),
				],
				total: 2,
			},
			...routes,
		})
		const path = tab ? `/routines/${ID}/${tab}` : `/routines/${ID}`
		await renderAtPath(
			withQueryClient(
				<>
					<RoutineDetailPage routineId={ID} tab={tab} />
					<Where />
				</>,
			),
			path,
			EXTRA,
		)
		await screen.findByRole('heading', {
			level: 1,
			name: 'Analyse board (hebdomadaire)',
		})
		return fetchMock
	}

	it('shows the header and overview by default', async () => {
		await open()
		expect(screen.queryByText(/Données de démonstration/)).toBeNull()
		expect(screen.getByText('Active')).toBeTruthy()
		expect(await screen.findByText(`${DEV.name} · Lun 06:30`)).toBeTruthy()
		expect(screen.getByText('Prochain passage')).toBeTruthy()
		expect(screen.getByText('Tâches créées')).toBeTruthy()
		expect(screen.getByText('4')).toBeTruthy()
		expect(screen.getByText('1 · 2')).toBeTruthy()
		expect(screen.getByText('Ignorer ce passage')).toBeTruthy()
		expect(screen.getByText('Rattraper une seule fois')).toBeTruthy()
		expect(screen.getByText('Analyser le board et documenter.')).toBeTruthy()
	})

	it('exposes one link per tab with aria-current on the active one', async () => {
		await open('executions')
		const nav = screen.getByRole('navigation', {
			name: 'Sections de la routine',
		})
		expect(
			within(nav)
				.getAllByRole('link')
				.map((l) => l.getAttribute('href')),
		).toEqual([
			`/routines/${ID}/apercu`,
			`/routines/${ID}/declencheurs`,
			`/routines/${ID}/executions`,
			`/routines/${ID}/reglages`,
		])
		expect(
			within(nav)
				.getByRole('link', { name: 'Passages' })
				.getAttribute('aria-current'),
		).toBe('page')
	})

	it('lists runs with labels, duration and task links', async () => {
		await open('executions')
		expect(await screen.findByText('terminée')).toBeTruthy()
		expect(screen.getByText('ignorée')).toBeTruthy()
		expect(screen.getByText('1 min 12')).toBeTruthy()
		expect(
			screen.getByText('La tâche précédente est encore ouverte'),
		).toBeTruthy()
		expect(
			screen.getByRole('link', { name: '6f1c2a90' }).getAttribute('href'),
		).toBe(`/tasks/${CARD}`)
	})

	it('pages through the runs', async () => {
		const user = userEvent.setup()
		const fetchMock = await open('executions', {
			[`GET /api/v1/routines/${ID}/runs`]: {
				items: [LAST],
				total: 45,
			},
		})
		await user.click(await screen.findByRole('button', { name: 'Suivants' }))
		await waitFor(() =>
			expect(
				calls(fetchMock, 'GET').some(
					({ url }) =>
						url.pathname === `/api/v1/routines/${ID}/runs` &&
						url.searchParams.get('offset') === '20' &&
						url.searchParams.get('limit') === '20',
				),
			).toBe(true),
		)
	})

	it('shows the empty state for a never-run routine', async () => {
		await open('executions', {
			[`GET /api/v1/routines/${ID}/runs`]: { items: [], total: 0 },
		})
		expect(await screen.findByText('Jamais lancée')).toBeTruthy()
	})

	it('launches now and opens the runs', async () => {
		const user = userEvent.setup()
		const fetchMock = await open(undefined, {
			[`POST /api/v1/routines/${ID}/launch`]: {
				status: 202,
				body: routineRun('run-9', { status: 'pending', origin: 'manual' }),
			},
		})
		await user.click(screen.getByRole('button', { name: 'Lancer maintenant' }))
		await waitFor(() =>
			expect(screen.getByTestId('where').textContent).toBe(
				`/routines/${ID}/executions`,
			),
		)
		expect(calls(fetchMock, 'POST')[0]?.url.pathname).toBe(
			`/api/v1/routines/${ID}/launch`,
		)
	})

	it('pauses the routine from the header', async () => {
		const user = userEvent.setup()
		const fetchMock = await open(undefined, {
			[`PATCH /api/v1/routines/${ID}`]: { ...ROUTINE, enabled: false },
		})
		await user.click(screen.getByRole('button', { name: 'Mettre en pause' }))
		await waitFor(() =>
			expect(calls(fetchMock, 'PATCH')[0]?.body).toEqual({ enabled: false }),
		)
	})

	it('edits a planning trigger', async () => {
		const user = userEvent.setup()
		const fetchMock = await open('declencheurs', {
			[`PATCH /api/v1/routines/${ID}/triggers/t1`]: ROUTINE,
		})
		await user.click(screen.getByRole('button', { name: 'Modifier' }))
		fireEvent.change(screen.getByLabelText('Heure'), {
			target: { value: '07:15' },
		})
		await user.click(screen.getByRole('checkbox', { name: 'Mer' }))
		await user.click(
			screen.getByRole('button', { name: 'Enregistrer le planning' }),
		)
		await waitFor(() =>
			expect(calls(fetchMock, 'PATCH')[0]?.body).toEqual({
				schedule: { frequency: 'weekly', time: '07:15', days: ['mon', 'wed'] },
			}),
		)
		expect(await screen.findByText('Planning enregistré')).toBeTruthy()
	})

	it('refuses an invalid cron before calling the API', async () => {
		const user = userEvent.setup()
		const fetchMock = await open('declencheurs')
		await user.click(screen.getByRole('button', { name: 'Modifier' }))
		await user.selectOptions(screen.getByLabelText('Fréquence'), 'cron')
		await user.type(screen.getByLabelText('Expression cron'), '* *')
		await user.click(
			screen.getByRole('button', { name: 'Enregistrer le planning' }),
		)
		expect(screen.getByText(/doit avoir 5 champs/)).toBeTruthy()
		expect(calls(fetchMock, 'PATCH')).toHaveLength(0)
	})

	it('adds a webhook and shows its token once', async () => {
		const user = userEvent.setup()
		const fetchMock = await open('declencheurs', {
			[`POST /api/v1/routines/${ID}/triggers`]: {
				status: 201,
				body: {
					routine: { ...ROUTINE, triggers: [PLANNING, WEBHOOK] },
					trigger_id: 't2',
					token: 'tok-secret',
					webhook_url: 'https://api.test/hooks/routines/tok-secret',
				},
			},
		})
		await user.selectOptions(
			screen.getByLabelText('Type de déclencheur'),
			'webhook',
		)
		await user.click(
			screen.getByRole('button', { name: 'Ajouter un déclencheur' }),
		)
		await waitFor(() =>
			expect(calls(fetchMock, 'POST')[0]?.body).toEqual({ kind: 'webhook' }),
		)
		const box = await screen.findByRole('region', { name: 'Accès au webhook' })
		expect(within(box).getByText('tok-secret')).toBeTruthy()
		expect(
			within(box).getByText('https://api.test/hooks/routines/tok-secret'),
		).toBeTruthy()
		expect(
			within(box).getByText(/le corps de la requête est ignoré/),
		).toBeTruthy()
		await user.click(
			within(box).getByRole('button', { name: "J'ai copié l'URL" }),
		)
		expect(
			screen.queryByRole('region', { name: 'Accès au webhook' }),
		).toBeNull()
	})

	it('adds a planning trigger', async () => {
		const user = userEvent.setup()
		const fetchMock = await open('declencheurs', {
			[`POST /api/v1/routines/${ID}/triggers`]: {
				status: 201,
				body: { routine: ROUTINE, trigger_id: 't3' },
			},
		})
		await user.click(
			screen.getByRole('button', { name: 'Ajouter un déclencheur' }),
		)
		await user.selectOptions(
			within(screen.getByRole('form', { name: 'Planning' })).getByLabelText(
				'Fréquence',
			),
			'every_hours',
		)
		await user.click(
			screen.getByRole('button', { name: 'Ajouter le planning' }),
		)
		await waitFor(() =>
			expect(calls(fetchMock, 'POST')[0]?.body).toEqual({
				kind: 'planning',
				schedule: { frequency: 'every_hours', hours: 24 },
			}),
		)
	})

	it('rotates a webhook token after confirmation', async () => {
		const user = userEvent.setup()
		const fetchMock = await open(
			'declencheurs',
			{
				[`POST /api/v1/routines/${ID}/triggers/t2/rotate-token`]: {
					routine: { ...ROUTINE, triggers: [PLANNING, WEBHOOK] },
					trigger_id: 't2',
					token: 'tok-new',
					webhook_url: 'https://api.test/hooks/routines/tok-new',
				},
			},
			{ ...ROUTINE, triggers: [PLANNING, WEBHOOK] },
		)
		expect(screen.getByText('••••••••k9Qz')).toBeTruthy()
		await user.click(screen.getByRole('button', { name: 'Régénérer le token' }))
		const dialog = await screen.findByRole('alertdialog')
		await user.click(within(dialog).getByRole('button', { name: 'Régénérer' }))
		await waitFor(() =>
			expect(calls(fetchMock, 'POST')[0]?.url.pathname).toBe(
				`/api/v1/routines/${ID}/triggers/t2/rotate-token`,
			),
		)
		expect(await screen.findByText('tok-new')).toBeTruthy()
	})

	it('toggles and removes a trigger', async () => {
		const user = userEvent.setup()
		const fetchMock = await open('declencheurs', {
			[`PATCH /api/v1/routines/${ID}/triggers/t1`]: ROUTINE,
			[`DELETE /api/v1/routines/${ID}/triggers/t1`]: {
				...ROUTINE,
				triggers: [],
			},
		})
		await user.click(
			screen.getByRole('switch', { name: 'Activer le planning' }),
		)
		await waitFor(() =>
			expect(calls(fetchMock, 'PATCH')[0]?.body).toEqual({ enabled: false }),
		)
		await user.click(screen.getByRole('button', { name: 'Retirer' }))
		const dialog = await screen.findByRole('alertdialog')
		await user.click(within(dialog).getByRole('button', { name: 'Retirer' }))
		await waitFor(() =>
			expect(calls(fetchMock, 'DELETE')[0]?.url.pathname).toBe(
				`/api/v1/routines/${ID}/triggers/t1`,
			),
		)
	})

	it('validates and saves only the changed settings', async () => {
		const user = userEvent.setup()
		const fetchMock = await open('reglages', {
			[`PATCH /api/v1/routines/${ID}`]: { ...ROUTINE, name: 'Bilan' },
		})
		const name = screen.getByLabelText('Nom')
		await user.clear(name)
		await user.click(screen.getByRole('button', { name: 'Enregistrer' }))
		expect(screen.getByText('Donne un nom à la routine.')).toBeTruthy()
		await user.type(name, 'Bilan')
		await user.selectOptions(
			screen.getByLabelText('Si la tâche précédente est encore ouverte'),
			'queue',
		)
		expect(screen.getByText(/attend que la tâche précédente/)).toBeTruthy()
		await user.click(screen.getByRole('button', { name: 'Enregistrer' }))
		await waitFor(() =>
			expect(calls(fetchMock, 'PATCH')[0]?.body).toEqual({
				name: 'Bilan',
				if_open: 'queue',
			}),
		)
		expect(await screen.findByText('Routine enregistrée')).toBeTruthy()
	})

	it('cancels edits', async () => {
		const user = userEvent.setup()
		await open('reglages')
		const name = screen.getByLabelText('Nom')
		await user.type(name, ' x')
		await user.click(screen.getByRole('button', { name: 'Annuler' }))
		expect((screen.getByLabelText('Nom') as HTMLInputElement).value).toBe(
			'Analyse board (hebdomadaire)',
		)
	})

	it('archives the routine', async () => {
		const user = userEvent.setup()
		const fetchMock = await open('reglages', {
			[`POST /api/v1/routines/${ID}/archive`]: { ...ROUTINE, archived: true },
		})
		await user.click(
			screen.getByRole('button', { name: 'Archiver la routine' }),
		)
		await waitFor(() =>
			expect(calls(fetchMock, 'POST')[0]?.url.pathname).toBe(
				`/api/v1/routines/${ID}/archive`,
			),
		)
	})

	it('reports an unknown routine', async () => {
		mockApi({ ...BASE })
		await renderAtPath(
			withQueryClient(<RoutineDetailPage routineId="nope" />),
			'/routines/nope',
			EXTRA,
		)
		expect(await screen.findByText('Routine introuvable')).toBeTruthy()
	})
})

describe('RoutineNewPage', () => {
	async function open(routes: Routes = {}) {
		const fetchMock = mockApi({
			...BASE,
			'POST /api/v1/routines': {
				status: 201,
				body: routine('new-id', { name: 'Bilan été' }),
			},
			...routes,
		})
		await renderAtPath(
			withQueryClient(
				<>
					<RoutineNewPage />
					<Where />
				</>,
			),
			'/routines/nouvelle',
			EXTRA,
		)
		await screen.findByRole('heading', { name: 'Nouvelle routine' })
		await screen.findByRole('option', { name: DEV.name })
		return fetchMock
	}

	it('refuses a missing name, prompt or agent', async () => {
		const user = userEvent.setup()
		const fetchMock = await open()
		await user.click(screen.getByRole('button', { name: 'Créer la routine' }))
		expect(screen.getByText('Donne un nom à la routine.')).toBeTruthy()
		await user.type(screen.getByLabelText('Nom'), 'Test')
		await user.type(screen.getByLabelText('Consigne'), 'Faire')
		await user.click(screen.getByRole('button', { name: 'Créer la routine' }))
		expect(screen.getByText(/Choisis l'agent/)).toBeTruthy()
		expect(calls(fetchMock, 'POST')).toHaveLength(0)
	})

	it('requires a weekday for weekly schedules', async () => {
		const user = userEvent.setup()
		await open()
		await user.type(screen.getByLabelText('Nom'), 'Test')
		await user.type(screen.getByLabelText('Consigne'), 'Faire')
		await user.selectOptions(screen.getByLabelText('Agent'), DEV.id)
		await user.click(screen.getByRole('checkbox', { name: 'Lun' }))
		await user.click(screen.getByRole('button', { name: 'Créer la routine' }))
		expect(screen.getByText('Choisis au moins un jour.')).toBeTruthy()
	})

	it('creates the routine and opens it', async () => {
		const user = userEvent.setup()
		const fetchMock = await open()
		await user.type(screen.getByLabelText('Nom'), 'Bilan été')
		expect((screen.getByLabelText('Slug') as HTMLInputElement).value).toBe(
			'bilan-ete',
		)
		await user.type(screen.getByLabelText('Consigne'), 'Faire le bilan')
		await user.selectOptions(screen.getByLabelText('Agent'), DEV.id)
		await user.selectOptions(screen.getByLabelText('Priorité'), 'high')
		await user.click(screen.getByRole('checkbox', { name: 'Mer' }))
		await user.selectOptions(
			screen.getByLabelText('Si un passage est manqué'),
			'skip',
		)
		await user.click(
			screen.getByRole('switch', { name: 'Écritures sans approbation' }),
		)
		await user.click(screen.getByRole('button', { name: 'Créer la routine' }))
		await waitFor(() =>
			expect(screen.getByTestId('where').textContent).toBe('/routines/new-id'),
		)
		expect(calls(fetchMock, 'POST')[0]?.body).toEqual({
			name: 'Bilan été',
			slug: 'bilan-ete',
			prompt: 'Faire le bilan',
			agent_id: DEV.id,
			project_id: null,
			priority: 'high',
			title_template: '{nom} · {date}',
			timezone: 'Europe/Paris',
			enabled: true,
			if_open: 'skip',
			if_missed: 'skip',
			allow_writes: true,
			schedule: { frequency: 'weekly', time: '08:00', days: ['mon', 'wed'] },
		})
	})

	it('creates an unplanned routine and shows API refusals', async () => {
		const user = userEvent.setup()
		const fetchMock = await open({
			'POST /api/v1/routines': {
				status: 409,
				body: { code: 'conflict', message: 'slug taken' },
			},
		})
		await user.type(screen.getByLabelText('Nom'), 'Bilan')
		await user.type(screen.getByLabelText('Consigne'), 'Faire')
		await user.selectOptions(screen.getByLabelText('Agent'), DEV.id)
		await user.click(screen.getByRole('switch', { name: 'Planifier' }))
		expect(screen.queryByLabelText('Fréquence')).toBeNull()
		await user.click(screen.getByRole('button', { name: 'Créer la routine' }))
		expect(
			await screen.findByText('Une routine porte déjà ce slug.'),
		).toBeTruthy()
		expect(calls(fetchMock, 'POST')[0]?.body.schedule).toBeNull()
	})
})
