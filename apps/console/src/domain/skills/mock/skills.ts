export type SkillScope = 'Espace de travail' | 'Intégré'

export interface DemoSkill {
	name: string
	scope: SkillScope
	description: string
	agents: number
}

export const DEMO_SKILLS: ReadonlyArray<DemoSkill> = [
	{
		name: 'humanizer',
		scope: 'Espace de travail',
		description:
			"Retire les marques d'écriture IA et guide la rédaction française naturelle.",
		agents: 21,
	},
	{
		name: 'para-memory-files',
		scope: 'Intégré',
		description:
			'Mémoire PARA en fichiers : faits durables, notes quotidiennes, synthèse hebdomadaire.',
		agents: 16,
	},
	{
		name: 'continue',
		scope: 'Espace de travail',
		description:
			"Mène un plan ou une spec jusqu'à un point fixe vérifié ; mode issue jusqu'à la MR relue.",
		agents: 12,
	},
	{
		name: 'ui-ux',
		scope: 'Espace de travail',
		description: 'Sonde de mise en page, captures, contrôle de contraste.',
		agents: 6,
	},
	{
		name: 'pennylane-depot',
		scope: 'Espace de travail',
		description:
			"Dépose une pièce d'achat dans Pennylane avec ses métadonnées.",
		agents: 3,
	},
	{
		name: 'gitlab-mr',
		scope: 'Intégré',
		description: 'Ouvre, relit et relance les merge requests.',
		agents: 5,
	},
]
