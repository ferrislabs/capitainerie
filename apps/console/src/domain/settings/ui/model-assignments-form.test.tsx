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
	MISTRAL_OCR,
	QWEN,
	SONNET,
	VOXTRAL,
	WHISPER,
} from '#/domain/settings/test-fixtures'
import { ModelAssignmentsForm } from '#/domain/settings/ui/model-assignments-form'

const QWEN_REF = { provider: 'openai_compatible', model: QWEN.id } as const
const BGE_REF = { provider: 'openai_compatible', model: BGE.id } as const

function setup(
	overrides: Partial<Parameters<typeof ModelAssignmentsForm>[0]> = {},
) {
	const handlers = {
		onChange: vi.fn(),
		onTranscriberChange: vi.fn(),
		onReaderChange: vi.fn(),
		onCaptainChange: vi.fn(),
		onIntegratorChange: vi.fn(),
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

	it('offers the transcriber only transcription models of mistral or openai_compatible', async () => {
		const anthropicTranscriber = {
			...SONNET,
			id: 'claude-whisper',
			capabilities: {
				chat: false,
				embeddings: false,
				transcription: true,
				ocr: false,
			},
		}
		const handlers = setup({
			models: [QWEN, BGE, SONNET, VOXTRAL, WHISPER, anthropicTranscriber],
		})
		const select = screen.getByLabelText('Transcripteur') as HTMLSelectElement

		expect(optionValues(select)).toEqual([
			'mistral:voxtral-mini-latest',
			'openai_compatible:whisper-large-v3',
		])
		expect(select.value).toBe('')
		expect(select.selectedOptions[0].textContent).toBe('Aucun transcripteur')

		await userEvent.selectOptions(select, 'mistral:voxtral-mini-latest')
		expect(handlers.onTranscriberChange).toHaveBeenCalledWith({
			provider: 'mistral',
			model: 'voxtral-mini-latest',
		})
	})

	it('clears the transcriber with the empty choice', async () => {
		const handlers = setup({
			models: [VOXTRAL],
			values: { transcriber: { provider: 'mistral', model: VOXTRAL.id } },
		})
		const select = screen.getByLabelText('Transcripteur') as HTMLSelectElement
		expect(select.value).toBe('mistral:voxtral-mini-latest')

		await userEvent.selectOptions(select, '')

		expect(handlers.onTranscriberChange).toHaveBeenCalledWith(undefined)
	})

	it('says when no model can transcribe', () => {
		setup({ models: [QWEN, BGE] })
		const select = screen.getByLabelText('Transcripteur') as HTMLSelectElement
		expect(select.disabled).toBe(true)
		expect(select.textContent).toContain('Aucun modèle de transcription')
		expect(
			screen.getByText(/Facultatif : sans lui, seuls les fichiers texte/),
		).toBeTruthy()
	})

	it('shows a saved transcriber the catalog dropped', () => {
		setup({
			models: [QWEN],
			values: { transcriber: { provider: 'mistral', model: 'voxtral-old' } },
		})

		const select = screen.getByLabelText('Transcripteur') as HTMLSelectElement
		expect(select.value).toBe('mistral:voxtral-old')
		expect(select.selectedOptions[0].textContent).toContain('indisponible')
	})

	it('offers the PDF reader only OCR models of mistral', async () => {
		const otherOcr = {
			...QWEN,
			id: 'some-ocr',
			capabilities: {
				chat: false,
				embeddings: false,
				transcription: false,
				ocr: true,
			},
		}
		const handlers = setup({
			models: [QWEN, BGE, SONNET, VOXTRAL, MISTRAL_OCR, otherOcr],
		})
		const select = screen.getByLabelText('Lecteur de PDF') as HTMLSelectElement

		expect(optionValues(select)).toEqual(['mistral:mistral-ocr-latest'])
		expect(select.value).toBe('')
		expect(select.selectedOptions[0].textContent).toBe('Aucun lecteur de PDF')

		await userEvent.selectOptions(select, 'mistral:mistral-ocr-latest')
		expect(handlers.onReaderChange).toHaveBeenCalledWith({
			provider: 'mistral',
			model: 'mistral-ocr-latest',
		})
		expect(handlers.onTranscriberChange).not.toHaveBeenCalled()
	})

	it('clears the reader with the empty choice', async () => {
		const handlers = setup({
			models: [MISTRAL_OCR],
			values: { reader: { provider: 'mistral', model: MISTRAL_OCR.id } },
		})
		const select = screen.getByLabelText('Lecteur de PDF') as HTMLSelectElement
		expect(select.value).toBe('mistral:mistral-ocr-latest')

		await userEvent.selectOptions(select, '')

		expect(handlers.onReaderChange).toHaveBeenCalledWith(undefined)
	})

	it('says when no model can read a PDF', () => {
		setup({ models: [QWEN, BGE] })
		const select = screen.getByLabelText('Lecteur de PDF') as HTMLSelectElement

		expect(select.disabled).toBe(true)
		expect(select.textContent).toContain('Aucun modèle OCR')
		expect(
			screen.getByText(/Facultatif : sans lui, les PDF sont refusés/),
		).toBeTruthy()
	})

	it('offers any chat model to the Captain, who defaults to the Rédacteur', async () => {
		const handlers = setup()
		const select = screen.getByLabelText('Capitaine') as HTMLSelectElement

		expect(optionValues(select)).toContain(`openai_compatible:${QWEN.id}`)
		expect(optionValues(select)).not.toContain(`openai_compatible:${BGE.id}`)
		expect(select.value).toBe('')
		expect(select.selectedOptions[0].textContent).toBe('Le modèle du Rédacteur')
		expect(
			screen.getByText(/sans lui, le modèle du Rédacteur sert/),
		).toBeTruthy()

		await userEvent.selectOptions(select, `openai_compatible:${QWEN.id}`)
		expect(handlers.onCaptainChange).toHaveBeenCalledWith(QWEN_REF)
		expect(handlers.onReaderChange).not.toHaveBeenCalled()
	})

	it('clears the Captain with the empty choice', async () => {
		const handlers = setup({ values: { captain: QWEN_REF } })
		const select = screen.getByLabelText('Capitaine') as HTMLSelectElement
		expect(select.value).toBe(`openai_compatible:${QWEN.id}`)

		await userEvent.selectOptions(select, '')

		expect(handlers.onCaptainChange).toHaveBeenCalledWith(undefined)
	})

	it('offers any chat model to the Intervenant, who defaults to the Rédacteur', async () => {
		const handlers = setup()
		const select = screen.getByLabelText('Intervenant') as HTMLSelectElement

		expect(optionValues(select)).toContain(`openai_compatible:${QWEN.id}`)
		expect(optionValues(select)).not.toContain(`openai_compatible:${BGE.id}`)
		expect(select.value).toBe('')
		expect(select.selectedOptions[0].textContent).toBe('Le modèle du Rédacteur')
		expect(
			screen.getByText(/Agit sur les Odoo et n8n des clients/),
		).toBeTruthy()

		await userEvent.selectOptions(select, `openai_compatible:${QWEN.id}`)
		expect(handlers.onIntegratorChange).toHaveBeenCalledWith(QWEN_REF)
		expect(handlers.onCaptainChange).not.toHaveBeenCalled()
	})

	it('clears the Intervenant with the empty choice', async () => {
		const handlers = setup({ values: { integrator: QWEN_REF } })
		const select = screen.getByLabelText('Intervenant') as HTMLSelectElement
		expect(select.value).toBe(`openai_compatible:${QWEN.id}`)

		await userEvent.selectOptions(select, '')

		expect(handlers.onIntegratorChange).toHaveBeenCalledWith(undefined)
	})
})
