export interface Adapter {
	name: string
	initial: string
	hue: number
}

export const ADAPTERS: ReadonlyArray<Adapter> = [
	{ name: 'Claude Code', initial: 'C', hue: 18 },
	{ name: 'Codex', initial: 'X', hue: 235 },
	{ name: 'Cursor', initial: 'Cu', hue: 0 },
	{ name: 'Cursor Cloud', initial: 'Cc', hue: 0 },
	{ name: 'Gemini CLI', initial: 'G', hue: 212 },
	{ name: 'Grok Build', initial: 'Gk', hue: 0 },
	{ name: 'Hermes Gateway', initial: 'Hg', hue: 40 },
	{ name: 'Hermes', initial: 'H', hue: 40 },
	{ name: 'Kimi Code', initial: 'K', hue: 222 },
	{ name: 'OpenCode', initial: 'O', hue: 0 },
	{ name: 'Paperclip Runner', initial: 'R', hue: 150 },
	{ name: 'Pi', initial: 'Pi', hue: 285 },
]

export const ROLES: ReadonlyArray<string> = [
	'CEO',
	'CTO',
	'CMO',
	'CFO',
	'PM',
	'Ingénieur',
	'Designer',
	'DevOps',
	'Sécurité',
	'Chercheur',
	'Général',
]

export const EFFORTS: ReadonlyArray<string> = [
	'Auto',
	'Faible',
	'Moyen',
	'Élevé',
]

export const WAKE_INTERVALS: ReadonlyArray<string> = [
	'Manuelle',
	'15 min',
	'1 h',
	'6 h',
	'24 h',
]

export const DIRECTION = 'Aucun (direction)'
export const HEAD_OF_TEAM = 'Chief of Staff'
