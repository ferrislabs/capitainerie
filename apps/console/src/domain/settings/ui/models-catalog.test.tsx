import { render, screen, within } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { BGE, QWEN, SONNET } from '#/domain/settings/test-fixtures'
import { ModelsCatalog } from '#/domain/settings/ui/models-catalog'

describe('ModelsCatalog', () => {
	it('lists models grouped by provider with capabilities, context and price', () => {
		render(
			<ModelsCatalog models={[QWEN, BGE, SONNET]} errors={[]} hasProviders />,
		)

		const rows = screen.getAllByRole('row')
		const text = rows.map((row) => row.textContent)
		expect(text[1]).toContain('Claude (Anthropic)')
		expect(text[3]).toContain('Compatible OpenAI')

		const sonnet = screen
			.getByText('claude-sonnet-5')
			.closest('tr') as HTMLElement
		expect(within(sonnet).getByText('Claude Sonnet 5')).toBeTruthy()
		expect(within(sonnet).getByText('Chat')).toBeTruthy()
		expect(within(sonnet).getByText('1M')).toBeTruthy()
		expect(within(sonnet).getByText('4,00 $ / 20,00 $')).toBeTruthy()

		const bge = screen.getByText('bge-m3:latest').closest('tr') as HTMLElement
		expect(within(bge).getByText('Embeddings')).toBeTruthy()
		expect(within(bge).queryByText('Chat')).toBeNull()
		expect(within(bge).getAllByText('—')).toHaveLength(2)

		const qwen = screen
			.getByText('qwen2.5:7b-instruct')
			.closest('tr') as HTMLElement
		expect(within(qwen).getByText('32k')).toBeTruthy()
	})

	it('keeps an unreachable provider visible with its error', () => {
		render(
			<ModelsCatalog
				models={[QWEN]}
				errors={[{ provider: 'mistral', message: 'connection refused' }]}
				hasProviders
			/>,
		)

		expect(screen.getByRole('alert').textContent).toBe(
			'Mistral injoignable : connection refused',
		)
		expect(screen.getByText('Injoignable')).toBeTruthy()
		expect(screen.getByText('Mistral')).toBeTruthy()
	})

	it('shows an empty state with the given action when no provider is configured', () => {
		render(
			<ModelsCatalog
				models={[]}
				errors={[]}
				hasProviders={false}
				emptyAction={
					<a href="/settings/providers">Configurer un fournisseur</a>
				}
			/>,
		)

		expect(screen.getByText('Aucun fournisseur configuré')).toBeTruthy()
		expect(
			screen.getByRole('link', { name: 'Configurer un fournisseur' }),
		).toBeTruthy()
		expect(screen.queryByRole('table')).toBeNull()
	})
})
