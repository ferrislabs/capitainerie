import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { ProviderKeyDialog } from '#/domain/settings/ui/provider-key-dialog'

function setup(
	overrides: Partial<Parameters<typeof ProviderKeyDialog>[0]> = {},
) {
	const handlers = {
		onApiKeyChange: vi.fn(),
		onBaseUrlChange: vi.fn(),
		onSubmit: vi.fn(),
		onClose: vi.fn(),
	}
	render(
		<ProviderKeyDialog
			open
			kind="mistral"
			isReplacing={false}
			apiKey=""
			baseUrl=""
			isPending={false}
			{...handlers}
			{...overrides}
		/>,
	)
	return handlers
}

describe('ProviderKeyDialog', () => {
	it('asks for a password-type key, without base URL for Mistral', async () => {
		const handlers = setup()
		const user = userEvent.setup()

		expect(
			screen.getByRole('dialog', { name: 'Configurer Mistral' }),
		).toBeTruthy()
		const key = screen.getByLabelText('Clé d’API') as HTMLInputElement
		expect(key.type).toBe('password')
		expect(key.value).toBe('')
		expect(screen.queryByLabelText('URL de base')).toBeNull()

		await user.type(key, 'x')
		expect(handlers.onApiKeyChange).toHaveBeenCalledWith('x')

		await user.click(screen.getByRole('button', { name: 'Enregistrer' }))
		expect(handlers.onSubmit).toHaveBeenCalledOnce()
	})

	it('shows the base URL with a placeholder, not a value, for Compatible OpenAI', () => {
		setup({ kind: 'openai_compatible' })

		const baseUrl = screen.getByLabelText('URL de base') as HTMLInputElement
		expect(baseUrl.value).toBe('')
		expect(baseUrl.placeholder).toBe('http://localhost:4120/v1')
		expect(screen.getByText('(optionnelle)')).toBeTruthy()
	})

	it('shows field and server errors and the pending state', () => {
		setup({
			isReplacing: true,
			apiKeyError: 'La clé d’API est obligatoire.',
			serverError: 'invalid api key',
			isPending: true,
		})

		expect(
			screen.getByRole('dialog', { name: 'Modifier Mistral' }),
		).toBeTruthy()
		expect(screen.getByText('La clé d’API est obligatoire.')).toBeTruthy()
		expect(screen.getByText('invalid api key')).toBeTruthy()
		const submit = screen.getByRole('button', {
			name: 'Enregistrement…',
		}) as HTMLButtonElement
		expect(submit.disabled).toBe(true)
	})

	it('renders nothing when closed', () => {
		setup({ open: false })
		expect(screen.queryByRole('dialog')).toBeNull()
	})
})
