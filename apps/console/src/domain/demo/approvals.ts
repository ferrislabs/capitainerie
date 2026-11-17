import type { DemoApproval } from '#/domain/demo/types'

/** Historique de départ : rien n'attend de décision. */
export const DEMO_APPROVALS: ReadonlyArray<DemoApproval> = [
	{
		id: 'APP-3',
		title: "Embauche de l'agent « Anouk (Design) »",
		detail: 'Nouvel agent de conception visuelle.',
		requestedBy: 'chief-of-staff',
		status: 'approved',
		at: '12 nov.',
		decidedBy: 'François-Guillaume Ribreau',
	},
	{
		id: 'APP-2',
		title: 'Dépassement de budget de Dev (cycle autonome) : +40 $',
		detail: 'Plafond mensuel atteint avant la fin du cycle.',
		requestedBy: 'dev',
		status: 'rejected',
		at: '3 nov.',
		decidedBy: 'François-Guillaume Ribreau',
	},
	{
		id: 'APP-1',
		title: 'Accès en écriture au Drive « compta/ »',
		detail: 'Archivage des pièces comptables sous compta/.',
		requestedBy: 'archibald',
		status: 'approved',
		at: '29 oct.',
		decidedBy: 'Baptiste Parmantier',
	},
]
