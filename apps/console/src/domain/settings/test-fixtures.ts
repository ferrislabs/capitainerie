import type { Schemas } from '#/api/api.client'

export function model(
	provider: Schemas.ProviderKindDto,
	id: string,
	overrides: Partial<Schemas.ModelResponse> = {},
): Schemas.ModelResponse {
	return {
		provider,
		id,
		display_name: id,
		capabilities: {
			chat: true,
			embeddings: false,
			transcription: false,
			ocr: false,
		},
		context_window: null,
		price: null,
		...overrides,
	}
}

export const QWEN = model('openai_compatible', 'qwen2.5:7b-instruct', {
	context_window: 32_768,
})
export const BGE = model('openai_compatible', 'bge-m3:latest', {
	capabilities: {
		chat: false,
		embeddings: true,
		transcription: false,
		ocr: false,
	},
})
export const SONNET = model('anthropic', 'claude-sonnet-5', {
	display_name: 'Claude Sonnet 5',
	context_window: 1_000_000,
	price: {
		input_per_mtok_micros: 4_000_000,
		output_per_mtok_micros: 20_000_000,
	},
})
export const CLAUDE_HAIKU = model('anthropic', 'claude-haiku-5', {
	display_name: 'Claude Haiku 5',
})
export const ANTHROPIC_OTHER = model('anthropic', 'legacy-model')
export const CLAUDE_LOOKALIKE = model('openai_compatible', 'claude-proxy')
export const MISTRAL_EMBED = model('mistral', 'mistral-embed', {
	capabilities: {
		chat: false,
		embeddings: true,
		transcription: false,
		ocr: false,
	},
})

export const VOXTRAL = model('mistral', 'voxtral-mini-latest', {
	capabilities: {
		chat: false,
		embeddings: false,
		transcription: true,
		ocr: false,
	},
})
export const WHISPER = model('openai_compatible', 'whisper-large-v3', {
	capabilities: {
		chat: false,
		embeddings: false,
		transcription: true,
		ocr: false,
	},
})

export const MISTRAL_OCR = model('mistral', 'mistral-ocr-latest', {
	capabilities: {
		chat: false,
		embeddings: false,
		transcription: false,
		ocr: true,
	},
})

export const COMPATIBLE_PROVIDER: Schemas.ProviderResponse = {
	kind: 'openai_compatible',
	base_url: 'http://localhost:4120/v1',
	preview: '',
	created_by: 'demo',
	updated_at: '2026-10-18T09:30:00Z',
}

export const ANTHROPIC_PROVIDER: Schemas.ProviderResponse = {
	kind: 'anthropic',
	base_url: null,
	preview: 'a1b2',
	created_by: 'ada',
	updated_at: '2026-10-17T14:05:00Z',
}
