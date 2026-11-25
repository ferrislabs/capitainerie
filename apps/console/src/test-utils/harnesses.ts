import type { Schemas } from '#/api/api.client'

export const HARNESS_LIST: Schemas.HarnessListResponse = {
	harnesses: [
		{
			id: 'claude-code',
			name: 'Claude Code',
			provider: 'anthropic',
			default_model: 'claude-sonnet-5-5',
			models: ['claude-opus-5', 'claude-sonnet-5-5', 'claude-haiku-5-5'],
		},
		{
			id: 'codex',
			name: 'Codex',
			provider: 'openai',
			default_model: 'gpt-6-sol',
			models: ['gpt-6-sol', 'gpt-6-mini'],
		},
		{
			id: 'mistral-vibe',
			name: 'Mistral Vibe',
			provider: 'mistral',
			default_model: 'devstral-2',
			models: ['devstral-2', 'mistral-large'],
		},
		{
			id: 'antigravity',
			name: 'Antigravity',
			provider: 'google',
			default_model: 'gemini-3-pro',
			models: ['gemini-3-pro', 'gemini-3-flash'],
		},
	],
}

export function personalKey(
	kind: Schemas.ProviderKindDto,
	preview = 'sk-…abcd',
): Schemas.UserProviderResponse {
	return {
		kind,
		preview,
		default_model: '',
		is_default: false,
		updated_at: '2026-11-01T00:00:00Z',
	}
}

export const HARNESS_ROUTES = {
	'GET /api/v1/harnesses': HARNESS_LIST,
	'GET /api/v1/me/providers': [personalKey('openai')],
	'GET /api/v1/providers': { providers: [] },
}
