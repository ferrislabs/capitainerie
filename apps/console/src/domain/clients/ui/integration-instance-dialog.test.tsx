import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { Schemas } from '#/api/api.client'
import { IntegrationInstanceDialog } from '#/domain/clients/ui/integration-instance-dialog'

const STORED: Schemas.IntegrationInstanceResponse = {
	id: 'i1',
	client_id: 'cl1',
	kind: 'odoo_jsonrpc',
	name: 'Odoo prod',
	environment: 'production',
	base_url: 'https://acme.odoo.com',
	database: 'acme',
	login: 'api@acme.test',
	secret_preview: '…c3d4',
	server_version: '17.0',
	checked_at: '2026-10-20T08:00:00Z',
	created_by: 'ada',
	created_at: '2026-10-16T09:00:00Z',
	updated_at: '2026-10-16T09:00:00Z',
	archived_at: null,
}

const HELP =
	'L’API externe d’Odoo Online demande une offre Custom. Pour la production, utilisez de préférence la clé d’un utilisateur Odoo en lecture seule.'

const NEW_KEY_HELP =
	'Une nouvelle clé est obligatoire quand l’adresse, la base ou l’identifiant change.'

function setup(
	overrides: Partial<Parameters<typeof IntegrationInstanceDialog>[0]> = {},
) {
	const onSubmit = vi.fn()
	const onClose = vi.fn()
	render(
		<IntegrationInstanceDialog
			open
			isPending={false}
			onSubmit={onSubmit}
			onClose={onClose}
			{...overrides}
		/>,
	)
	return { onSubmit, onClose, user: userEvent.setup() }
}

describe('IntegrationInstanceDialog', () => {
	it('offers the three types, the three environments and a password field', () => {
		setup()
		const dialog = screen.getByRole('dialog')

		expect(
			within(within(dialog).getByLabelText('Type'))
				.getAllByRole('option')
				.map((option) => option.textContent),
		).toEqual(['Odoo 19 et plus', 'Odoo 18 et moins', 'n8n'])
		expect(
			within(within(dialog).getByLabelText('Environnement'))
				.getAllByRole('option')
				.map((option) => option.textContent),
		).toEqual(['Production', 'Staging', 'Développement'])
		expect(
			within(dialog)
				.getByLabelText('Clé API / mot de passe')
				.getAttribute('type'),
		).toBe('password')
	})

	it('shows the base as optional for Odoo 19 and no login', () => {
		setup()

		expect(screen.getByLabelText('Base (facultative)')).toBeTruthy()
		expect(screen.queryByLabelText('Identifiant')).toBeNull()
		expect(screen.getByText(HELP)).toBeTruthy()
	})

	it('shows neither base nor login nor the Odoo help for n8n', async () => {
		const { user } = setup()

		await user.selectOptions(screen.getByLabelText('Type'), 'n8n')

		expect(screen.queryByLabelText(/^Base/)).toBeNull()
		expect(screen.queryByLabelText('Identifiant')).toBeNull()
		expect(screen.queryByText(HELP)).toBeNull()
	})

	it('requires the base and the login for Odoo 18 and earlier', async () => {
		const { user, onSubmit } = setup()
		await user.selectOptions(screen.getByLabelText('Type'), 'odoo_jsonrpc')
		await user.type(screen.getByLabelText('Nom'), 'Odoo prod')
		await user.type(screen.getByLabelText('URL'), 'https://acme.odoo.com')
		await user.type(screen.getByLabelText('Clé API / mot de passe'), 'pw')

		await user.click(screen.getByRole('button', { name: 'Ajouter' }))

		expect(await screen.findByText('La base est obligatoire.')).toBeTruthy()
		expect(screen.getByText('L’identifiant est obligatoire.')).toBeTruthy()
		expect(onSubmit).not.toHaveBeenCalled()

		await user.type(screen.getByLabelText('Base'), 'acme')
		await user.type(screen.getByLabelText('Identifiant'), 'api@acme.test')
		await user.click(screen.getByRole('button', { name: 'Ajouter' }))

		expect(onSubmit).toHaveBeenCalledWith({
			kind: 'odoo_jsonrpc',
			name: 'Odoo prod',
			environment: 'production',
			baseUrl: 'https://acme.odoo.com',
			database: 'acme',
			login: 'api@acme.test',
			secret: 'pw',
		})
	})

	it('refuses an empty form on the name, the address and the key', async () => {
		const { user, onSubmit } = setup()

		await user.click(screen.getByRole('button', { name: 'Ajouter' }))

		expect(await screen.findByText('Le nom est obligatoire.')).toBeTruthy()
		expect(screen.getByText('L’adresse est obligatoire.')).toBeTruthy()
		expect(screen.getByText('La clé est obligatoire.')).toBeTruthy()
		expect(onSubmit).not.toHaveBeenCalled()
	})

	it('clears the key from the form once submitted', async () => {
		const { user, onSubmit } = setup()
		await user.type(screen.getByLabelText('Nom'), 'Odoo prod')
		await user.type(screen.getByLabelText('URL'), 'https://acme.odoo.com')
		const key = screen.getByLabelText('Clé API / mot de passe')
		await user.type(key, 'secret-value')

		await user.click(screen.getByRole('button', { name: 'Ajouter' }))

		expect(onSubmit).toHaveBeenCalledTimes(1)
		expect(onSubmit.mock.calls[0][0].secret).toBe('secret-value')
		await waitFor(() => expect((key as HTMLInputElement).value).toBe(''))
	})

	it('says the connection is being tested while saving', () => {
		setup({ isPending: true })

		const button = screen.getByRole('button', { name: 'Test de la connexion…' })
		expect((button as HTMLButtonElement).disabled).toBe(true)
		expect((screen.getByLabelText('Nom') as HTMLInputElement).disabled).toBe(
			true,
		)
	})

	it('shows the error it is given', () => {
		setup({ errorMessage: 'Connexion impossible : timeout' })

		expect(screen.getByRole('alert').textContent).toContain(
			'Connexion impossible : timeout',
		)
	})

	it('edits with the stored values, a fixed type and an empty key', async () => {
		const { user, onSubmit } = setup({ instance: STORED })

		expect((screen.getByLabelText('Type') as HTMLSelectElement).disabled).toBe(
			true,
		)
		expect((screen.getByLabelText('Type') as HTMLSelectElement).value).toBe(
			'odoo_jsonrpc',
		)
		expect((screen.getByLabelText('Nom') as HTMLInputElement).value).toBe(
			'Odoo prod',
		)
		expect((screen.getByLabelText('Base') as HTMLInputElement).value).toBe(
			'acme',
		)
		expect(
			(screen.getByLabelText('Clé API / mot de passe') as HTMLInputElement)
				.value,
		).toBe('')
		expect(
			screen.getByText(/Laissez vide pour garder la clé actuelle/),
		).toBeTruthy()

		await user.click(screen.getByRole('button', { name: 'Enregistrer' }))

		expect(onSubmit).toHaveBeenCalledWith(
			expect.objectContaining({ name: 'Odoo prod', secret: '' }),
		)
	})

	it('keeps the key optional when the path alone changes', async () => {
		const { user, onSubmit } = setup({ instance: STORED })
		const url = screen.getByLabelText('URL')
		await user.clear(url)
		await user.type(url, 'https://acme.odoo.com/odoo')

		expect(screen.queryByText(NEW_KEY_HELP)).toBeNull()
		await user.click(screen.getByRole('button', { name: 'Enregistrer' }))

		expect(onSubmit).toHaveBeenCalledWith(
			expect.objectContaining({
				baseUrl: 'https://acme.odoo.com/odoo',
				secret: '',
			}),
		)
	})

	it.each([
		['URL', 'https://other.odoo.com'],
		['Base', 'other'],
		['Identifiant', 'bot'],
	])('requires a new key when the %s changes', async (label, value) => {
		const { user, onSubmit } = setup({ instance: STORED })
		const field = screen.getByLabelText(label)
		await user.clear(field)
		await user.type(field, value)

		expect(screen.getByText(NEW_KEY_HELP)).toBeTruthy()
		expect(screen.queryByText(/Laissez vide pour garder/)).toBeNull()
		expect(
			screen
				.getByLabelText('Clé API / mot de passe')
				.getAttribute('aria-required'),
		).toBe('true')

		await user.click(screen.getByRole('button', { name: 'Enregistrer' }))

		expect(await screen.findByText('La clé est obligatoire.')).toBeTruthy()
		expect(onSubmit).not.toHaveBeenCalled()

		await user.type(screen.getByLabelText('Clé API / mot de passe'), 'new-key')
		await user.click(screen.getByRole('button', { name: 'Enregistrer' }))

		expect(onSubmit).toHaveBeenCalledWith(
			expect.objectContaining({ secret: 'new-key' }),
		)
	})

	it('goes back to an optional key when the change is reverted', async () => {
		const { user } = setup({ instance: STORED })
		const database = screen.getByLabelText('Base')
		await user.clear(database)
		await user.type(database, 'other')
		expect(screen.getByText(NEW_KEY_HELP)).toBeTruthy()

		await user.clear(database)
		await user.type(database, 'acme')

		expect(screen.queryByText(NEW_KEY_HELP)).toBeNull()
		expect(
			screen.getByText(/Laissez vide pour garder la clé actuelle/),
		).toBeTruthy()
	})

	it('never asks for the new key sentence when adding', () => {
		setup()

		expect(screen.queryByText(NEW_KEY_HELP)).toBeNull()
	})

	it('closes on cancel', async () => {
		const { user, onClose } = setup()

		await user.click(screen.getByRole('button', { name: 'Annuler' }))

		expect(onClose).toHaveBeenCalledTimes(1)
	})
})
