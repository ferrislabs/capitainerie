import type { Schemas } from '#/api/api.client'
import { slugify } from '#/components/markdown'

export type Artifact = Schemas.ArtifactSummaryResponse
export type ArtifactDetail = Schemas.ArtifactResponse
export type ArtifactKind = Schemas.ArtifactKindDto

export const ARTIFACT_KINDS: ReadonlyArray<{
	kind: ArtifactKind
	label: string
}> = [
	{ kind: 'markdown', label: 'Document' },
	{ kind: 'json', label: 'Données JSON' },
	{ kind: 'link', label: 'Lien' },
]

export const kindLabel = (kind: ArtifactKind): string =>
	ARTIFACT_KINDS.find((item) => item.kind === kind)?.label ?? kind

/** JSON indenté ; le texte tel quel s'il n'est pas du JSON valide. */
export function prettyJson(content: string): string {
	try {
		return JSON.stringify(JSON.parse(content), null, 2)
	} catch {
		return content
	}
}

/** URL ouvrable dans un nouvel onglet : http ou https uniquement. */
export function safeUrl(url: string | null | undefined): string | null {
	if (!url) return null
	try {
		const parsed = new URL(url)
		return parsed.protocol === 'http:' || parsed.protocol === 'https:'
			? parsed.toString()
			: null
	} catch {
		return null
	}
}

/** Nom du fichier téléchargé : le titre en minuscules sans accents, `.json` ou `.md`. */
export function artifactFileName(title: string, kind: ArtifactKind): string {
	return `${slugify(title) || 'artifact'}.${kind === 'json' ? 'json' : 'md'}`
}
