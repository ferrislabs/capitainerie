import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import {
	ANTHROPIC_PROVIDER,
	COMPATIBLE_PROVIDER,
} from '#/domain/settings/test-fixtures'
import { ProvidersList } from '#/domain/settings/ui/providers-list'

function setup(overrides: Partial<Parameters<typeof ProvidersList>[0]> = {}) {
	const handlers = {
		onConfigure: vi.fn(),
		onTest: vi.fn(),
		onDelete: vi.fn(),
	}
	render(
		<ProvidersList
			providers={[ANTHROPIC_PROVIDER, COMPATIBLE_PROVIDER]}
			isAdmin
			currentSubject="demo"
			testResults={{}}
			{...handlers}
			{...overrides}
		/>,
	)
	return handlers
}

function row(name: string) {
	return screen.getByRole('listitem', { name })
}

describe('ProvidersList', () => {
	it('shows every provider kind with its state', () => {
		setup()

		const anthropic = row('Claude (Anthropic)')
		expect(within(anthropic).getByText('Configuré')).toBeTruthy()
		expect(within(anthropic).getByText('…a1b2')).toBeTruthy()
		expect(within(anthropic).getByText('ada')).toBeTruthy()
		expect(within(anthropic).getByText(/console\.anthropic\.com/)).toBeTruthy()

		const mistral = row('Mistral')
		expect(within(mistral).getByText('Non configuré')).toBeTruthy()
		expect(within(mistral).getByText(/console\.mistral\.ai/)).toBeTruthy()

		const compatible = row('Compatible OpenAI')
		expect(within(compatible).getByText('Sans clé')).toBeTruthy()
		expect(within(compatible).getByText(/par vous/)).toBeTruthy()
		expect(
			within(compatible).getByText('http://localhost:4120/v1'),
		).toBeTruthy()
		expect(
			within(compatible).getByText(/Ollama local ne demande pas de clé/),
		).toBeTruthy()
	})

	it('offers the admin actions per state', async () => {
		const handlers = setup()
		const user = userEvent.setup()

		expect(
			within(row('Mistral')).getByRole('button', { name: /Configurer/ }),
		).toBeTruthy()
		expect(
			within(row('Mistral')).queryByRole('button', { name: /Tester/ }),
		).toBeNull()

		await user.click(
			within(row('Claude (Anthropic)')).getByRole('button', {
				name: /Remplacer la clé/,
			}),
		)
		expect(handlers.onConfigure).toHaveBeenCalledWith('anthropic')

		await user.click(
			screen.getByRole('button', { name: 'Tester Compatible OpenAI' }),
		)
		expect(handlers.onTest).toHaveBeenCalledWith('openai_compatible')

		await user.click(
			screen.getByRole('button', { name: 'Supprimer Claude (Anthropic)' }),
		)
		expect(handlers.onDelete).toHaveBeenCalledWith('anthropic')
	})

	it('is read-only for members', () => {
		setup({ isAdmin: false })

		expect(screen.queryAllByRole('button')).toHaveLength(0)
		expect(
			within(row('Claude (Anthropic)')).getByText('Configuré'),
		).toBeTruthy()
	})

	it('shows test outcomes and the pending test', () => {
		setup({
			testingKind: 'anthropic',
			testResults: {
				openai_compatible: { ok: true, modelCount: 2 },
				anthropic: { ok: false, error: 'invalid x-api-key' },
			},
		})

		expect(
			within(row('Compatible OpenAI')).getByText(
				'Connexion réussie : 2 modèles annoncés.',
			),
		).toBeTruthy()
		expect(
			within(row('Claude (Anthropic)')).getByRole('alert').textContent,
		).toContain('invalid x-api-key')
		const testing = screen.getByRole('button', {
			name: 'Tester Claude (Anthropic)',
		}) as HTMLButtonElement
		expect(testing.disabled).toBe(true)
		expect(testing.textContent).toContain('Test…')
	})
})
