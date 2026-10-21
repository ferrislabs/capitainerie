import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from '#/api'
import type { Schemas } from '#/api/api.client'
import { IntegrationInstancesSection } from '#/domain/clients/ui/integration-instances-section'
import { ADMIN_ROLE } from '#/domain/identity/roles'
import { createTestClient, json } from '#/domain/settings/hooks/test-utils'
import { clearAuth, setAuth } from '#/store/auth.store'

function tokenWithRoles(roles: Array<string>) {
	const payload = btoa(JSON.stringify({ realm_access: { roles } }))
	return `e30.${payload.replace(/=+$/, '')}.sig`
}

const CHECKED_AT = '2026-10-20T08:00:00Z'

const dateFormat = new Intl.DateTimeFormat('fr-FR', {
	dateStyle: 'short',
	timeStyle: 'short',
})

const ODOO: Schemas.IntegrationInstanceResponse = {
	id: 'i1',
	client_id: 'cl1',
	kind: 'odoo_json2',
	name: 'Odoo prod',
	environment: 'production',
	base_url: 'https://acme.odoo.com',
	database: null,
	login: null,
	secret_preview: '…c3d4',
	server_version: '19.0',
	checked_at: CHECKED_AT,
	created_by: 'ada',
	created_at: '2026-10-16T09:00:00Z',
	updated_at: '2026-10-16T09:00:00Z',
	archived_at: null,
}

const OLD_ODOO: Schemas.IntegrationInstanceResponse = {
	...ODOO,
	id: 'i2',
	kind: 'odoo_jsonrpc',
	name: 'Odoo recette',
	environment: 'staging',
	base_url: 'https://recette.acme.test',
	database: 'acme',
	login: 'api@acme.test',
	secret_preview: '…9f8e',
	server_version: '17.0',
}

const N8N: Schemas.IntegrationInstanceResponse = {
	...ODOO,
	id: 'i3',
	kind: 'n8n',
	name: 'Flux',
	environment: 'development',
	base_url: 'https://n8n.acme.test',
	secret_preview: '…77aa',
	server_version: '1',
	checked_at: null,
}

type Route = (init: RequestInit, url: URL) => Response | Promise<Response>

const LIST = '/api/v1/clients/cl1/integrations'

describe('IntegrationInstancesSection', () => {
	const fetchMock = vi.fn()
	let harness: ReturnType<typeof createTestClient>

	function serve(table: Record<string, Route>) {
		fetchMock.mockImplementation((url: URL, init: RequestInit) => {
			const route = table[`${init.method} ${url.pathname}`]
			if (!route) throw new Error(`unexpected ${init.method} ${url.pathname}`)
			return Promise.resolve(route(init, url))
		})
	}

	function calls(method: string, path: string) {
		return fetchMock.mock.calls.filter(
			([url, init]) =>
				(url as URL).pathname === path &&
				(init as RequestInit).method === method,
		)
	}

	function bodyOf(method: string, path: string) {
		const init = calls(method, path)[0][1] as RequestInit
		return JSON.parse(String(init.body))
	}

	function listing(items: Schemas.IntegrationInstanceResponse[]) {
		return { [`GET ${LIST}`]: () => json(200, { items }) }
	}

	function asAdmin() {
		setAuth({ accessToken: tokenWithRoles([ADMIN_ROLE]) })
	}

	function mount(archived = false) {
		return render(
			<IntegrationInstancesSection clientId="cl1" archived={archived} />,
			{ wrapper: harness.wrapper },
		)
	}

	async function addAnInstance(
		user: ReturnType<typeof userEvent.setup>,
		secret = 'key-123',
	) {
		await user.click(
			await screen.findByRole('button', { name: 'Ajouter une intégration' }),
		)
		const dialog = await screen.findByRole('dialog')
		await user.type(within(dialog).getByLabelText('Nom'), ' Odoo prod ')
		await user.type(
			within(dialog).getByLabelText('URL'),
			'https://acme.odoo.com',
		)
		await user.type(
			within(dialog).getByLabelText('Clé API / mot de passe'),
			secret,
		)
		await user.click(within(dialog).getByRole('button', { name: 'Ajouter' }))
		return dialog
	}

	beforeEach(() => {
		harness = createTestClient()
		api.setBaseUrl('http://api.test')
		vi.stubGlobal('fetch', fetchMock)
		setAuth({ accessToken: tokenWithRoles(['member']) })
	})

	afterEach(() => {
		fetchMock.mockReset()
		vi.unstubAllGlobals()
		clearAuth()
		harness.client.clear()
	})

	it('lists one row per instance with its figures and no secret field', async () => {
		serve(listing([ODOO, OLD_ODOO, N8N]))
		const { container } = mount()

		const row = (await screen.findByText('Odoo prod')).closest(
			'tr',
		) as HTMLElement
		expect(within(row).getByText('Odoo 19 et plus')).toBeTruthy()
		expect(within(row).getByText('Production')).toBeTruthy()
		expect(within(row).getByText('https://acme.odoo.com')).toBeTruthy()
		expect(within(row).getByText('19.0')).toBeTruthy()
		expect(
			within(row).getByText(dateFormat.format(new Date(CHECKED_AT))),
		).toBeTruthy()
		expect(within(row).getByText('…c3d4')).toBeTruthy()
		const old = screen.getByText('Odoo recette').closest('tr') as HTMLElement
		expect(within(old).getByText('Odoo 18 et moins')).toBeTruthy()
		expect(within(old).getByText('Staging')).toBeTruthy()
		const flow = screen.getByText('Flux').closest('tr') as HTMLElement
		expect(within(flow).getByText('Développement')).toBeTruthy()
		expect(within(flow).getByText('n8n 1.x')).toBeTruthy()
		expect(within(flow).getByText('Jamais testée')).toBeTruthy()
		expect(container.querySelector('input')).toBeNull()
		expect(screen.queryByLabelText(/Clé/)).toBeNull()
	})

	it('shows n8n 2.x for a second-generation n8n', async () => {
		serve(listing([{ ...N8N, server_version: '2' }]))
		mount()

		expect(await screen.findByText('n8n 2.x')).toBeTruthy()
	})

	it('shows the production environment in a danger tone', async () => {
		serve(listing([ODOO, N8N]))
		mount()

		expect((await screen.findByText('Production')).dataset.variant).toBe(
			'danger',
		)
		expect(screen.getByText('Développement').dataset.variant).not.toBe('danger')
	})

	it('says so when the client has no integration', async () => {
		serve(listing([]))
		mount()

		expect(await screen.findByText('Aucune intégration')).toBeTruthy()
	})

	it('says so when the list cannot be loaded', async () => {
		serve({
			[`GET ${LIST}`]: () =>
				json(500, { code: 'internal', status: 500, message: 'boom' }),
		})
		mount()

		expect(
			await screen.findByText('Impossible de charger les intégrations.'),
		).toBeTruthy()
	})

	it('is read-only for a member', async () => {
		serve(listing([ODOO]))
		mount()

		await screen.findByText('Odoo prod')
		expect(screen.queryByRole('button')).toBeNull()
	})

	it('gives an administrator the four actions', async () => {
		asAdmin()
		serve(listing([ODOO]))
		mount()

		const row = (await screen.findByText('Odoo prod')).closest(
			'tr',
		) as HTMLElement
		expect(
			within(row).getByRole('button', { name: 'Tester la connexion' }),
		).toBeTruthy()
		expect(within(row).getByRole('button', { name: 'Modifier' })).toBeTruthy()
		expect(within(row).getByRole('button', { name: 'Archiver' })).toBeTruthy()
		expect(
			screen.getByRole('button', { name: 'Ajouter une intégration' }),
		).toBeTruthy()
	})

	it('cannot add an integration to an archived client', async () => {
		asAdmin()
		serve(listing([ODOO]))
		mount(true)

		const add = await screen.findByRole('button', {
			name: 'Ajouter une intégration',
		})
		expect((add as HTMLButtonElement).disabled).toBe(true)
		expect(
			(
				within(
					(await screen.findByText('Odoo prod')).closest('tr') as HTMLElement,
				).getByRole('button', { name: 'Modifier' }) as HTMLButtonElement
			).disabled,
		).toBe(false)
	})

	it('creates an instance, says the connection is tested meanwhile and keeps the key out of the caches', async () => {
		asAdmin()
		let release: (response: Response) => void = () => undefined
		serve({
			...listing([]),
			[`POST ${LIST}`]: () =>
				new Promise<Response>((resolve) => {
					release = resolve
				}),
		})
		mount()
		const user = userEvent.setup()

		await addAnInstance(user, 'secret-value-42')

		expect(
			await screen.findByRole('button', { name: 'Test de la connexion…' }),
		).toBeTruthy()
		expect(bodyOf('POST', LIST)).toEqual({
			kind: 'odoo_json2',
			name: 'Odoo prod',
			environment: 'production',
			base_url: 'https://acme.odoo.com',
			secret: 'secret-value-42',
		})
		release(json(201, ODOO))

		await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull())
		await waitFor(() => expect(calls('GET', LIST).length).toBeGreaterThan(1))
		expect(
			JSON.stringify(
				harness.client
					.getQueryCache()
					.getAll()
					.map((query) => [query.queryKey, query.state.data]),
			),
		).not.toContain('secret-value-42')
		await waitFor(() =>
			expect(
				JSON.stringify(
					harness.client
						.getMutationCache()
						.getAll()
						.map((mutation) => mutation.state.variables),
				),
			).not.toContain('secret-value-42'),
		)
	})

	it.each([
		[422, 'connection_failed', 'timeout', 'Connexion impossible : timeout'],
		[
			422,
			'version_mismatch',
			'no JSON-2',
			'Ce serveur ne sert pas ce protocole. Choisissez l’autre type Odoo.',
		],
		[
			422,
			'network_blocked',
			'10.0.0.5',
			'Adresse refusée par la garde réseau (adresse privée ou http)',
		],
		[
			409,
			'instance_name_taken',
			'Odoo prod',
			'Ce client a déjà une intégration de ce nom',
		],
		[422, 'invalid', 'the name is too long', 'the name is too long'],
		[
			403,
			'forbidden',
			'forbidden',
			'Seuls les administrateurs de l’organisation peuvent gérer les intégrations.',
		],
	])('explains a refused creation (%i %s)', async (status, code, message, french) => {
		asAdmin()
		serve({
			...listing([]),
			[`POST ${LIST}`]: () => json(status, { code, status, message }),
		})
		mount()
		const user = userEvent.setup()

		const dialog = await addAnInstance(user)

		expect(await within(dialog).findByText(french)).toBeTruthy()
		expect(screen.getByRole('dialog')).toBe(dialog)
	})

	it('edits an instance and leaves the key alone when the field stays empty', async () => {
		asAdmin()
		serve({
			...listing([ODOO]),
			'PATCH /api/v1/integration-instances/i1': () =>
				json(200, { ...ODOO, name: 'Odoo production' }),
		})
		mount()
		const user = userEvent.setup()

		await user.click(await screen.findByRole('button', { name: 'Modifier' }))
		const dialog = await screen.findByRole('dialog')
		expect(
			(within(dialog).getByLabelText('Nom') as HTMLInputElement).value,
		).toBe('Odoo prod')
		expect(
			(within(dialog).getByLabelText('Type') as HTMLSelectElement).disabled,
		).toBe(true)
		await user.clear(within(dialog).getByLabelText('Nom'))
		await user.type(within(dialog).getByLabelText('Nom'), 'Odoo production')
		await user.click(
			within(dialog).getByRole('button', { name: 'Enregistrer' }),
		)

		await waitFor(() =>
			expect(calls('PATCH', '/api/v1/integration-instances/i1')).toHaveLength(
				1,
			),
		)
		expect(bodyOf('PATCH', '/api/v1/integration-instances/i1')).toEqual({
			name: 'Odoo production',
		})
		await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull())
	})

	it('sends the new key when one is typed', async () => {
		asAdmin()
		serve({
			...listing([ODOO]),
			'PATCH /api/v1/integration-instances/i1': () => json(200, ODOO),
		})
		mount()
		const user = userEvent.setup()

		await user.click(await screen.findByRole('button', { name: 'Modifier' }))
		const dialog = await screen.findByRole('dialog')
		await user.type(
			within(dialog).getByLabelText('Clé API / mot de passe'),
			'new-key',
		)
		await user.click(
			within(dialog).getByRole('button', { name: 'Enregistrer' }),
		)

		await waitFor(() =>
			expect(calls('PATCH', '/api/v1/integration-instances/i1')).toHaveLength(
				1,
			),
		)
		expect(bodyOf('PATCH', '/api/v1/integration-instances/i1')).toEqual({
			secret: 'new-key',
		})
	})

	it('closes the edit dialog without a request when nothing changed', async () => {
		asAdmin()
		serve(listing([ODOO]))
		mount()
		const user = userEvent.setup()

		await user.click(await screen.findByRole('button', { name: 'Modifier' }))
		const dialog = await screen.findByRole('dialog')
		await user.click(
			within(dialog).getByRole('button', { name: 'Enregistrer' }),
		)

		await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull())
		expect(calls('PATCH', '/api/v1/integration-instances/i1')).toHaveLength(0)
	})

	it('shows the refusal of a save in the edit dialog', async () => {
		asAdmin()
		serve({
			...listing([ODOO]),
			'PATCH /api/v1/integration-instances/i1': () =>
				json(422, {
					code: 'connection_failed',
					status: 422,
					message: 'invalid key',
				}),
		})
		mount()
		const user = userEvent.setup()

		await user.click(await screen.findByRole('button', { name: 'Modifier' }))
		const dialog = await screen.findByRole('dialog')
		await user.type(
			within(dialog).getByLabelText('Clé API / mot de passe'),
			'bad',
		)
		await user.click(
			within(dialog).getByRole('button', { name: 'Enregistrer' }),
		)

		expect(
			await within(dialog).findByText('Connexion impossible : invalid key'),
		).toBeTruthy()
	})

	it('tests the connection and shows the version and the warning', async () => {
		asAdmin()
		serve({
			...listing([N8N]),
			'POST /api/v1/integration-instances/i3/check': () =>
				json(200, {
					ok: true,
					server_version: '2',
					warning: 'The API key has write access',
					instance: { ...N8N, server_version: '2', checked_at: CHECKED_AT },
				}),
		})
		mount()
		const user = userEvent.setup()

		await user.click(
			await screen.findByRole('button', { name: 'Tester la connexion' }),
		)

		expect(await screen.findByText(/Connexion réussie/)).toBeTruthy()
		expect(screen.getAllByText(/n8n 2\.x/).length).toBeGreaterThan(0)
		expect(screen.getByText('The API key has write access')).toBeTruthy()
		await waitFor(() => expect(calls('GET', LIST).length).toBeGreaterThan(1))
	})

	it('tests the connection and shows why it failed', async () => {
		asAdmin()
		serve({
			...listing([ODOO]),
			'POST /api/v1/integration-instances/i1/check': () =>
				json(200, {
					ok: false,
					error: { code: 'connection_failed', message: 'bad key' },
					instance: ODOO,
				}),
		})
		mount()
		const user = userEvent.setup()

		await user.click(
			await screen.findByRole('button', { name: 'Tester la connexion' }),
		)

		expect(
			await screen.findByText('Connexion impossible : bad key'),
		).toBeTruthy()
		expect(screen.queryByText(/Connexion réussie/)).toBeNull()
	})

	it('says the connection is being tested while it runs', async () => {
		asAdmin()
		serve({
			...listing([ODOO]),
			'POST /api/v1/integration-instances/i1/check': () =>
				new Promise<Response>(() => undefined),
		})
		mount()
		const user = userEvent.setup()

		await user.click(
			await screen.findByRole('button', { name: 'Tester la connexion' }),
		)

		expect(
			await screen.findByRole('button', { name: 'Test de la connexion…' }),
		).toBeTruthy()
	})

	it('asks for a confirmation before archiving', async () => {
		asAdmin()
		serve({
			...listing([ODOO]),
			'DELETE /api/v1/integration-instances/i1': () =>
				json(200, { ...ODOO, archived_at: CHECKED_AT }),
		})
		mount()
		const user = userEvent.setup()

		await user.click(await screen.findByRole('button', { name: 'Archiver' }))
		const dialog = await screen.findByRole('alertdialog')
		expect(within(dialog).getByText(/Odoo prod/)).toBeTruthy()
		expect(calls('DELETE', '/api/v1/integration-instances/i1')).toHaveLength(0)
		await user.click(within(dialog).getByRole('button', { name: 'Archiver' }))

		await waitFor(() =>
			expect(calls('DELETE', '/api/v1/integration-instances/i1')).toHaveLength(
				1,
			),
		)
		await waitFor(() => expect(screen.queryByRole('alertdialog')).toBeNull())
		await waitFor(() => expect(calls('GET', LIST).length).toBeGreaterThan(1))
	})

	it('archives nothing when the confirmation is cancelled', async () => {
		asAdmin()
		serve(listing([ODOO]))
		mount()
		const user = userEvent.setup()

		await user.click(await screen.findByRole('button', { name: 'Archiver' }))
		const dialog = await screen.findByRole('alertdialog')
		await user.click(within(dialog).getByRole('button', { name: 'Annuler' }))

		await waitFor(() => expect(screen.queryByRole('alertdialog')).toBeNull())
		expect(calls('DELETE', '/api/v1/integration-instances/i1')).toHaveLength(0)
	})
})
