import type { Artifact } from '#/domain/artifacts/mock/artifacts'

const fold = (text: string): string =>
	text
		.normalize('NFD')
		.replace(/\p{Diacritic}/gu, '')
		.toLowerCase()

export function filterArtifacts(
	query: string,
	artifacts: ReadonlyArray<Artifact>,
): Artifact[] {
	const needle = fold(query.trim())
	if (!needle) return [...artifacts]
	return artifacts.filter((artifact) =>
		fold(`${artifact.name} ${artifact.agent} ${artifact.type}`).includes(
			needle,
		),
	)
}
