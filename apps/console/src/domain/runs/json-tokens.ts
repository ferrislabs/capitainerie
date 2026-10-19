export type JsonTokenKind =
	| 'key'
	| 'string'
	| 'number'
	| 'literal'
	| 'punct'
	| 'space'

export interface JsonToken {
	kind: JsonTokenKind
	text: string
}

const TOKEN_PATTERN =
	/("(?:\\.|[^"\\])*")(\s*:)?|-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?|true|false|null|[{}[\],:]|\s+/g

export function prettyJson(value: unknown): string {
	return JSON.stringify(value, null, 2) ?? 'null'
}

export function tokenizeJson(text: string): JsonToken[] {
	const tokens: JsonToken[] = []
	for (const match of text.matchAll(TOKEN_PATTERN)) {
		const [whole, quoted, colon] = match
		if (quoted !== undefined) {
			if (colon === undefined) {
				tokens.push({ kind: 'string', text: quoted })
				continue
			}
			tokens.push({ kind: 'key', text: quoted })
			const gap = colon.slice(0, colon.length - 1)
			if (gap) tokens.push({ kind: 'space', text: gap })
			tokens.push({ kind: 'punct', text: ':' })
		} else if (/^\s/.test(whole)) {
			tokens.push({ kind: 'space', text: whole })
		} else if (/^[{}[\],:]$/.test(whole)) {
			tokens.push({ kind: 'punct', text: whole })
		} else if (/^(true|false|null)$/.test(whole)) {
			tokens.push({ kind: 'literal', text: whole })
		} else {
			tokens.push({ kind: 'number', text: whole })
		}
	}
	return tokens
}
