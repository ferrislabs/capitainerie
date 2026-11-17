/**
 * Les pages sans backend se signalent par la pastille « Démo » de la barre du haut.
 * Ce texte reste pour les lecteurs d'écran, sans rien ajouter à la mise en page.
 */
export function DemoBanner() {
	return (
		<output className="sr-only">
			Données de démonstration. Cette page n'est pas encore reliée au backend :
			rien n'est enregistré et tout est perdu au rechargement.
		</output>
	)
}
