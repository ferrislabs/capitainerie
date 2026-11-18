import type { Schemas } from '#/api/api.client'

export type Skill = Schemas.SkillResponse

export interface SkillDraft {
	slug: string
	name: string
	description: string
	content: string
	source: string
}

export const SLUG_MAX = 60
const SLUG_PATTERN = /^[a-z0-9]+(?:-[a-z0-9]+)*$/
const URL_PATTERN = /^https?:\/\/\S+$/i

export const EMPTY_DRAFT: SkillDraft = {
	slug: '',
	name: '',
	description: '',
	content: '',
	source: 'manual',
}

export function draftOf(skill: Skill): SkillDraft {
	return {
		slug: skill.slug,
		name: skill.name,
		description: skill.description,
		content: skill.content,
		source: skill.source,
	}
}

export function filterSkills(
	skills: ReadonlyArray<Skill>,
	query: string,
): Skill[] {
	const needle = query.trim().toLowerCase()
	if (!needle) return [...skills]
	return skills.filter((s) =>
		[s.slug, s.name, s.description].some((field) =>
			field.toLowerCase().includes(needle),
		),
	)
}

/** Message d'erreur du formulaire, ou `null` si le brouillon est valide. */
export function validateDraft(draft: SkillDraft): string | null {
	const slug = draft.slug.trim()
	if (!slug) return 'Le slug est obligatoire.'
	if (slug.length > SLUG_MAX)
		return `Le slug ne doit pas dépasser ${SLUG_MAX} caractères.`
	if (!SLUG_PATTERN.test(slug))
		return 'Le slug doit être en kebab-case : minuscules, chiffres et tirets (ex. revue-contrat).'
	if (!draft.name.trim()) return 'Le nom est obligatoire.'
	const source = draft.source.trim()
	if (source && source !== 'manual' && !URL_PATTERN.test(source))
		return 'La source doit être « manual » ou une URL http(s).'
	return null
}

export function createBody(draft: SkillDraft): Schemas.CreateSkillRequest {
	return {
		slug: draft.slug.trim(),
		name: draft.name.trim(),
		description: draft.description.trim(),
		content: draft.content,
		source: draft.source.trim() || 'manual',
	}
}

/** Seuls les champs modifiés partent dans le PATCH. */
export function updateBody(
	skill: Skill,
	draft: SkillDraft,
): Schemas.UpdateSkillRequest {
	const next = createBody(draft)
	const body: Schemas.UpdateSkillRequest = {}
	if (next.slug !== skill.slug) body.slug = next.slug
	if (next.name !== skill.name) body.name = next.name
	if (next.description !== skill.description)
		body.description = next.description
	if (next.content !== skill.content) body.content = next.content
	if (next.source !== skill.source) body.source = next.source
	return body
}

export function sourceLabel(source: string): string {
	if (source === 'manual') return 'Manuelle'
	try {
		return new URL(source).host
	} catch {
		return source
	}
}

export function agentsUsing<T extends { skills: ReadonlyArray<string> }>(
	agents: ReadonlyArray<T>,
	slug: string,
): T[] {
	return agents.filter((a) => a.skills.includes(slug))
}

export function describeSkillError(error: unknown): string {
	const status = (error as { status?: number } | null)?.status
	if (status === 409) return 'Une compétence porte déjà ce slug.'
	if (status === 422)
		return 'Compétence refusée : slug en kebab-case (60 caractères max), nom requis, source « manual » ou URL http(s).'
	if (status === 403) return "Tu n'as pas le droit de modifier les compétences."
	if (status === 404) return "Cette compétence n'existe plus."
	return "L'enregistrement a échoué. Réessaie."
}
