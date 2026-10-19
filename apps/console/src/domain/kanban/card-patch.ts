import type { Schemas } from '#/api/api.client'

export type CardPatch = Omit<Schemas.PatchCardRequest, 'expected_version'>

export const MAX_CRITERIA = 20
export const MAX_CRITERION_CHARS = 300
export const MAX_LABELS = 10
export const MAX_LABEL_CHARS = 30

export function toggleCriterion(
	criteria: ReadonlyArray<Schemas.CriterionDto>,
	index: number,
): Schemas.CriterionDto[] {
	return criteria.map((criterion, position) =>
		position === index ? { ...criterion, done: !criterion.done } : criterion,
	)
}

export function renameCriterion(
	criteria: ReadonlyArray<Schemas.CriterionDto>,
	index: number,
	text: string,
): Schemas.CriterionDto[] {
	return criteria.map((criterion, position) =>
		position === index ? { ...criterion, text: text.trim() } : criterion,
	)
}

export function removeCriterion(
	criteria: ReadonlyArray<Schemas.CriterionDto>,
	index: number,
): Schemas.CriterionDto[] {
	return criteria.filter((_, position) => position !== index)
}

export function addCriterion(
	criteria: ReadonlyArray<Schemas.CriterionDto>,
	text: string,
): Schemas.CriterionDto[] | undefined {
	const clean = text.trim()
	if (clean === '' || criteria.length >= MAX_CRITERIA) return undefined
	return [...criteria, { text: clean, done: false }]
}

export function addLabel(
	labels: ReadonlyArray<string>,
	label: string,
): string[] | undefined {
	const clean = label.trim()
	if (clean === '' || labels.length >= MAX_LABELS) return undefined
	const known = labels.some(
		(existing) =>
			existing.toLocaleLowerCase('fr-FR') === clean.toLocaleLowerCase('fr-FR'),
	)
	return known ? undefined : [...labels, clean]
}

export function removeLabel(
	labels: ReadonlyArray<string>,
	label: string,
): string[] {
	return labels.filter((existing) => existing !== label)
}
