import type { DemoProject } from '#/domain/demo/types'

export const DEMO_PROJECTS: ReadonlyArray<DemoProject> = [
	{
		name: 'Site getnatalia.com',
		description: 'SEO, pages support, guides partenaires, diff gate.',
		progress: 34,
		openTasks: 12,
		agentId: 'seo-website-optimizer',
	},
	{
		name: 'Archivage comptable',
		description: 'Pièces reçues, dépôt Pennylane, rangement du Drive.',
		progress: 71,
		openTasks: 4,
		agentId: 'archibald',
	},
	{
		name: 'Appliance on-prem',
		description:
			"Chaîne d'appro, licence ed25519, RLS Postgres, observabilité.",
		progress: 48,
		openTasks: 9,
		agentId: 'dev',
	},
	{
		name: 'Marketing et notoriété',
		description:
			'Newsletter hebdo, plan de la semaine, contenu issu des appels de vente.',
		progress: 56,
		openTasks: 7,
		agentId: 'constance',
	},
	{
		name: 'Conformité',
		description: 'Revue ISO/IEC 42001 hebdomadaire, registres et preuves.',
		progress: 82,
		openTasks: 2,
		agentId: 'compliance-iso-iec-42001',
	},
]
