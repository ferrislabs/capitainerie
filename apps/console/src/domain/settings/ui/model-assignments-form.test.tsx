import { render, screen } from '@testing-library/react'
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
import { chooseValue, listValues, textOf } from '#/test-utils/combobox'

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

describe('ModelAssignmentsForm', () => {
	it('lists only compatible models per role, grouped by provider', async () => {
		setup()
		const user = userEvent.setup()

		for (const label of ['Extracteur', 'Juge de doublons', 'Rédacteur']) {
			const select = screen.getByLabelText(label)
			expect(await listValues(user, select)).toEqual([
				'anthropic:claude-sonnet-5',
				'openai_compatible:qwen2.5:7b-instruct',
			])
		}
		const embeddings = screen.getByLabelText('Embeddings')
		await user.click(embeddings)
		expect(
			screen
				.getAllByRole('group')
				.map((group) => group.textContent?.slice(0, 20)),
		).toHaveLength(2)
		expect(screen.getByRole('group', { name: 'Mistral' })).toBeTruthy()
		expect(
			screen.getByRole('group', { name: 'Compatible OpenAI' }),
		).toBeTruthy()
		await user.keyboard('{Escape}')
		expect(await listValues(user, embeddings)).toEqual([
			'mistral:mistral-embed',
			'openai_compatible:bge-m3:latest',
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

		await chooseValue(
			user,
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
			(screen.getByLabelText('Extracteur') as HTMLInputElement).disabled,
		).toBe(true)
		expect(textOf(screen.getByLabelText('Extracteur'))).toContain(
			'qwen2.5:7b-instruct',
		)
	})

	it('keeps a saved model that the catalog no longer lists', () => {
		setup({
			values: { writer: { provider: 'anthropic', model: 'claude-old' } },
		})

		const writer = screen.getByLabelText('Rédacteur')
		expect(textOf(writer)).toBe('claude-old (Claude (Anthropic), indisponible)')
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

		const embeddings = screen.getByLabelText('Embeddings') as HTMLInputElement
		expect(embeddings.disabled).toBe(true)
		expect(embeddings.placeholder).toContain('Aucun modèle d’embeddings')
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
		const user = userEvent.setup()
		const select = screen.getByLabelText('Exécutant')

		expect(await listValues(user, select)).toEqual([
			'claude-sonnet-5',
			'claude-haiku-5',
		])
		expect(
			screen.getByText(
				'Claude Code, sur les cartes confiées. Modèles Claude uniquement.',
			),
		).toBeTruthy()

		await chooseValue(user, select, 'claude-haiku-5')
		expect(handlers.onExecutorChange).toHaveBeenCalledWith('claude-haiku-5')
	})

	it('shows the saved executor, even when the catalog dropped it', () => {
		setup({ values: { executor: 'claude-old' } })

		const executor = screen.getByLabelText('Exécutant')
		expect(textOf(executor)).toBe('claude-old (indisponible)')
	})

	it('says when the catalog has no Claude model, and locks members out', () => {
		setup({ models: [QWEN, BGE] })
		const executor = screen.getByLabelText('Exécutant') as HTMLInputElement
		expect(executor.disabled).toBe(true)
		expect(executor.placeholder).toContain('Aucun modèle Claude')
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
		const user = userEvent.setup()
		const select = screen.getByLabelText('Transcripteur')

		expect(await listValues(user, select)).toEqual([
			'mistral:voxtral-mini-latest',
			'openai_compatible:whisper-large-v3',
		])
		expect(textOf(select)).toBe('Aucun transcripteur')

		await chooseValue(user, select, 'mistral:voxtral-mini-latest')
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
		const user = userEvent.setup()
		const select = screen.getByLabelText('Transcripteur')
		expect(textOf(select)).toContain('voxtral-mini-latest')

		await chooseValue(user, select, '')

		expect(handlers.onTranscriberChange).toHaveBeenCalledWith(undefined)
	})

	it('says when no model can transcribe', () => {
		setup({ models: [QWEN, BGE] })
		const select = screen.getByLabelText('Transcripteur') as HTMLInputElement
		expect(select.disabled).toBe(true)
		expect(textOf(select)).toContain('Aucun modèle de transcription')
		expect(
			screen.getByText(/Facultatif : sans lui, seuls les fichiers texte/),
		).toBeTruthy()
	})

	it('shows a saved transcriber the catalog dropped', () => {
		setup({
			models: [QWEN],
			values: { transcriber: { provider: 'mistral', model: 'voxtral-old' } },
		})

		const select = screen.getByLabelText('Transcripteur')
		expect(textOf(select)).toBe('voxtral-old (Mistral, indisponible)')
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
		const user = userEvent.setup()
		const select = screen.getByLabelText('Lecteur de PDF')

		expect(await listValues(user, select)).toEqual([
			'mistral:mistral-ocr-latest',
		])
		expect(textOf(select)).toBe('Aucun lecteur de PDF')

		await chooseValue(user, select, 'mistral:mistral-ocr-latest')
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
		const user = userEvent.setup()
		const select = screen.getByLabelText('Lecteur de PDF')
		expect(textOf(select)).toContain('mistral-ocr-latest')

		await chooseValue(user, select, '')

		expect(handlers.onReaderChange).toHaveBeenCalledWith(undefined)
	})

	it('says when no model can read a PDF', () => {
		setup({ models: [QWEN, BGE] })
		const select = screen.getByLabelText('Lecteur de PDF') as HTMLInputElement

		expect(select.disabled).toBe(true)
		expect(textOf(select)).toContain('Aucun modèle OCR')
		expect(
			screen.getByText(/Facultatif : sans lui, les PDF sont refusés/),
		).toBeTruthy()
	})

	it('offers any chat model to the Captain, who defaults to the Rédacteur', async () => {
		const handlers = setup()
		const user = userEvent.setup()
		const select = screen.getByLabelText('Capitaine')

		const values = await listValues(user, select)
		expect(values).toContain(`openai_compatible:${QWEN.id}`)
		expect(values).not.toContain(`openai_compatible:${BGE.id}`)
		expect(textOf(select)).toBe('Le modèle du Rédacteur')
		expect(
			screen.getByText(/sans lui, le modèle du Rédacteur sert/),
		).toBeTruthy()

		await chooseValue(user, select, `openai_compatible:${QWEN.id}`)
		expect(handlers.onCaptainChange).toHaveBeenCalledWith(QWEN_REF)
		expect(handlers.onReaderChange).not.toHaveBeenCalled()
	})

	it('clears the Captain with the empty choice', async () => {
		const handlers = setup({ values: { captain: QWEN_REF } })
		const user = userEvent.setup()
		const select = screen.getByLabelText('Capitaine')
		expect(textOf(select)).toContain(QWEN.id)

		await chooseValue(user, select, '')

		expect(handlers.onCaptainChange).toHaveBeenCalledWith(undefined)
	})
})
