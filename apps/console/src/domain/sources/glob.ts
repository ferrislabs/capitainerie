export interface GlobSource {
	patterns: ReadonlyArray<string>
	exclusions: ReadonlyArray<string>
}

export type TestVerdict =
	| { kind: 'excluded'; by: string }
	| { kind: 'match'; pattern: string }
	| { kind: 'none' }

export function globToRegExp(pattern: string): RegExp {
	const body = pattern
		.split('*')
		.map((part) => part.replace(/[.+?^${}()|[\]\\]/g, '\\$&'))
		.join('.+')
	return new RegExp(`^${body}$`, 'i')
}

export function testValue(
	source: GlobSource,
	value: string,
): TestVerdict | null {
	const v = value.trim().toLowerCase()
	if (!v) return null
	const by = source.exclusions.find((e) => globToRegExp(e).test(v))
	if (by !== undefined) return { kind: 'excluded', by }
	const pattern = source.patterns.find((p) => globToRegExp(p).test(v))
	return pattern === undefined ? { kind: 'none' } : { kind: 'match', pattern }
}
