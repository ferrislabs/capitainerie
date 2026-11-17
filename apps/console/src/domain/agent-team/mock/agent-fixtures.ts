import type { DemoAgent } from '#/domain/demo'
import { shortName } from '#/domain/demo'

export const SECRETS: ReadonlyArray<string> = [
	'gitlab-com-natalia',
	'chatwoot-api',
	'pennylane-api',
	'grafana-readonly',
]

export const LEGAL_INSTR = `# Veille légale (Sous-domaines) · Natalia

Tu es déclenché par la source « Nouveaux sous-domaines sensibles » : un nom d'hôte iam.* ou auth.* vient d'apparaître.

## Mission

1. Extraire le domaine racine du nom d'hôte reçu.
2. Interroger RDAP : titulaire, registrar, date de création. Si le titulaire est masqué, le noter et continuer.
3. Retrouver l'entreprise : annuaire des entreprises, RNE (INPI), Infogreffe. Rapprocher le nom, le site et l'adresse.
4. Relever : raison sociale, SIREN/SIRET, forme juridique, capital, siège, dirigeants, effectif, code NAF.
5. Contrôler le BODACC : procédures collectives, radiation.
6. Déposer le dossier dans la tâche, sources citées. Passer en revue si le rapprochement est incertain.

## Règles

- Un rapprochement repose sur au moins deux indices concordants (RDAP + site, ou adresse + nom).
- Ne jamais inventer un identifiant légal. Écrire « non trouvé » plutôt que deviner.
- Aucune donnée personnelle de dirigeant au-delà de ce que publie le registre.`

export function defaultInstructions(agent: DemoAgent): string {
	if (agent.id === 'veille-legale') return LEGAL_INSTR
	return `# ${shortName(agent.name)} · Natalia

Tu es l'agent « ${agent.role} » de Natalia. ${agent.description}.

## Mission

Tenir ton périmètre, documenter ce que tu observes, proposer plutôt que décider. Un verdict repose sur une preuve (fichier, job CI, réponse HTTP), jamais sur une supposition.

## Règles

- Au réveil, suis d'abord la skill Paperclip (heartbeat).
- Aucun secret en clair dans un commentaire ou un artifact.
- En cas de doute, escalade au Chief of Staff.`
}

export const DEFAULT_REVISIONS = [
	{
		label: 'Rév. 14',
		author: 'François-Guillaume Ribreau',
		when: 'il y a 3 j',
		changed: 'instructions',
	},
	{
		label: 'Rév. 13',
		author: 'Baptiste Parmantier',
		when: 'il y a 9 j',
		changed: 'modèle, effort de réflexion',
	},
	{
		label: 'Rév. 12',
		author: 'François-Guillaume Ribreau',
		when: 'il y a 21 j',
		changed: 'variables',
	},
] as const

export const DEFAULT_KEYS = [
	{
		name: 'Heartbeat CI',
		preview: 'pcp_7f3a…c91e',
		created: '2 nov. 2026',
		lastUsed: 'il y a 5 h',
	},
] as const
