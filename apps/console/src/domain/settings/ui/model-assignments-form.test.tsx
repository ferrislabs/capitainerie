import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { ADMIN_ONLY_MODELS } from '#/domain/settings/providers'
import {
	ANTHROPIC_OTHER,
	BGE,
	CLAUDE_HAIKU,
	CLAUDE_LOOKALIKE,
	MISTRAL_EMBED,
	QWEN,
	SONNET,
} from '#/domain/settings/test-fixtures'
import { ModelAssignmentsForm } from '#/domain/settings/ui/model-assignments-form'

const QWEN_REF = { provider: 'openai_compatible', model: QWEN.id } as const
const BGE_REF = { provider: 'openai_compatible', model: BGE.id } as const

function setup(
	overrides: Partial<Parameters<typeof ModelAssignmentsForm>[0]> = {},
) {
	const handlers = {
		onChange: vi.fn(),
		onExecutorChange: vi.fn(),
		onSubmit: vi.fn(),
	}
	render(
		<ModelAssignmentsForm
			models={[QWEN, BGE, SONNET, MISTRAL_EMBED]}
			values={{}}
			usesWorkerDefault={false}
			isAdmin
			readOnlyMessage={ADMIN_ONLY_MODELS}
			canSubmit={false}
			isPending={false}
			embeddingsChanged={false}
			{...handlers}
			{...overrides}
		/>,
	)
	return handlers
}

function optionValues(select: HTMLElement) {
	return within(select)
		.getAllByRole('option')
		.map((option) => (option as HTMLOptionElement).value)
		.filter(Boolean)
}

describe('ModelAssignmentsForm', () => {
	it('lists only compatible models per role, grouped by provider', () => {
		setup()

		for (const label of ['Extracteur', 'Juge de doublons', 'Rédacteur']) {
			const select = screen.getByLabelText(label)
			expect(optionValues(select)).toEqual([
				'anthropic:claude-sonnet-5',
				'openai_compatible:qwen2.5:7b-instruct',
			])
		}
		const embeddings = screen.getByLabelText('Embeddings')
		expect(optionValues(embeddings)).toEqual([
			'mistral:mistral-embed',
			'openai_compatible:bge-m3:latest',
		])
		const groups = embeddings.querySelectorAll('optgroup')
		expect([...groups].map((group) => group.label)).toEqual([
			'Mistral',
			'Compatible OpenAI',
		])
	})

	it('reports a role change and submits', async () => {
		const handlers = setup({
			values: {
				extractor: QWEN_REF,
				judge: QWEN_REF,
				writer: QWEN_REF,
				embeddings: BGE_REF,
			},
			canSubmit: true,
		})
		const user = userEvent.setup()

		await user.selectOptions(
			screen.getByLabelText('Extracteur'),
			'anthropic:claude-sonnet-5',
		)
		expect(handlers.onChange).toHaveBeenCalledWith('extractor', {
			provider: 'anthropic',
			model: 'claude-sonnet-5',
		})

		await user.click(screen.getByRole('button', { name: 'Enregistrer' }))
		expect(handlers.onSubmit).toHaveBeenCalledOnce()
	})

	it('explains the worker default when nothing is assigned', () => {
		setup({ usesWorkerDefault: true })

		expect(
			screen.getByText(/Configuration par défaut du worker \(Ollama local\)/),
		).toBeTruthy()
		expect(
			(screen.getByRole('button', { name: 'Enregistrer' }) as HTMLButtonElement)
				.disabled,
		).toBe(true)
	})

	it('is read-only for members', () => {
		setup({ isAdmin: false, values: { extractor: QWEN_REF } })

		expect(screen.getByText(ADMIN_ONLY_MODELS)).toBeTruthy()
		expect(screen.queryByRole('button')).toBeNull()
		expect(
			(screen.getByLabelText('Extracteur') as HTMLSelectElement).disabled,
		).toBe(true)
		expect(
			(screen.getByLabelText('Extracteur') as HTMLSelectElement).value,
		).toBe('openai_compatible:qwen2.5:7b-instruct')
	})

	it('keeps a saved model that the catalog no longer lists', () => {
		setup({
			values: { writer: { provider: 'anthropic', model: 'claude-old' } },
		})

		const writer = screen.getByLabelText('Rédacteur') as HTMLSelectElement
		expect(writer.value).toBe('anthropic:claude-old')
		expect(writer.selectedOptions[0].textContent).toContain('indisponible')
	})

	it('shows the reindex warning, the error and the success message', () => {
		setup({
			embeddingsChanged: true,
			errorMessage: 'provider mistral is not configured',
			successMessage: 'Modèles enregistrés.',
			isPending: true,
		})

		expect(
			screen.getByText(/Les cartes seront réindexées au prochain run/),
		).toBeTruthy()
		expect(screen.getByRole('alert').textContent).toBe(
			'provider mistral is not configured',
		)
		expect(screen.getByText('Modèles enregistrés.')).toBeTruthy()
		expect(screen.getByRole('button', { name: 'Enregistrement…' })).toBeTruthy()
	})

	it('says when no compatible model exists', () => {
		setup({ models: [QWEN] })

		const embeddings = screen.getByLabelText('Embeddings') as HTMLSelectElement
		expect(embeddings.disabled).toBe(true)
		expect(embeddings.textContent).toContain('Aucun modèle d’embeddings')
	})

	it('offers only Claude models to the executor and reports the choice', async () => {
		const handlers = setup({
			models: [
				QWEN,
				BGE,
				SONNET,
				CLAUDE_HAIKU,
				ANTHROPIC_OTHER,
				CLAUDE_LOOKALIKE,
			],
		})
		const select = screen.getByLabelText('Exécutant')

		expect(optionValues(select)).toEqual(['claude-sonnet-5', 'claude-haiku-5'])
		expect(
			screen.getByText(
				'Claude Code, sur les cartes confiées. Modèles Claude uniquement.',
			),
		).toBeTruthy()

		await userEvent.selectOptions(select, 'claude-haiku-5')
		expect(handlers.onExecutorChange).toHaveBeenCalledWith('claude-haiku-5')
	})

	it('shows the saved executor, even when the catalog dropped it', () => {
		setup({ values: { executor: 'claude-old' } })

		const executor = screen.getByLabelText('Exécutant') as HTMLSelectElement
		expect(executor.value).toBe('claude-old')
		expect(executor.selectedOptions[0].textContent).toContain('indisponible')
	})

	it('says when the catalog has no Claude model, and locks members out', () => {
		setup({ models: [QWEN, BGE] })
		const executor = screen.getByLabelText('Exécutant') as HTMLSelectElement
		expect(executor.disabled).toBe(true)
		expect(executor.textContent).toContain('Aucun modèle Claude')
	})
})
