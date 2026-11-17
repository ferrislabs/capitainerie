export type ArtifactType = 'Document' | 'Tableau de bord' | 'Page' | 'Feuille'

export interface Artifact {
	id: string
	name: string
	agent: string
	type: ArtifactType
	updated: string
}

export const ARTIFACTS: ReadonlyArray<Artifact> = [
	{
		id: 'plan-s47',
		name: 'Plan de la semaine 47',
		agent: 'Constance (Marketing)',
		type: 'Document',
		updated: 'il y a 3 h',
	},
	{
		id: 'iso-42001',
		name: 'Rapport de conformité ISO/IEC 42001',
		agent: 'Compliance ISO/IEC 42001',
		type: 'Document',
		updated: 'hier',
	},
	{
		id: 'latence-voix',
		name: 'Latence des appels de voix',
		agent: 'Henri (Observabilité)',
		type: 'Tableau de bord',
		updated: 'il y a 2 j',
	},
	{
		id: 'revue-agents',
		name: 'Revue hebdomadaire des agents',
		agent: 'Chief of Staff',
		type: 'Document',
		updated: 'lun.',
	},
	{
		id: 'schema-appro',
		name: "Schéma de la chaîne d'appro",
		agent: 'Anouk (Design)',
		type: 'Page',
		updated: 'il y a 4 j',
	},
	{
		id: 'inventaire-drive',
		name: 'Inventaire du Drive',
		agent: 'Archibald (Archivage comptable)',
		type: 'Feuille',
		updated: 'il y a 5 j',
	},
]
