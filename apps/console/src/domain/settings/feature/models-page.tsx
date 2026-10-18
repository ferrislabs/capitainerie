import { Cpu } from 'lucide-react'
import { PageHeader } from '#/components/layout/page-header'
import { ComingSoon } from '#/domain/settings/ui/coming-soon'

export function ModelsPage() {
	return (
		<>
			<PageHeader
				title="Modèles"
				description="Le modèle qui sert chaque rôle d'agent : extraction, juge, rédaction et embeddings."
			/>
			<ComingSoon
				icon={Cpu}
				title="Choisir un modèle par rôle"
				summary="Vous pourrez parcourir le catalogue des modèles annoncés par vos fournisseurs et assigner un modèle à chaque rôle d'agent."
				points={[
					'Catalogue des modèles disponibles avec vos clés, prix et fenêtre de contexte quand ils sont connus.',
					'Un modèle par rôle : extractor, judge, writer, embeddings.',
					'Le choix est figé dans chaque run au dépôt : un run en file garde ses modèles.',
				]}
			/>
		</>
	)
}
