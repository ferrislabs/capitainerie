import { KeyRound } from 'lucide-react'
import { PageHeader } from '#/components/layout/page-header'
import { ComingSoon } from '#/domain/settings/ui/coming-soon'

export function ProvidersPage() {
	return (
		<>
			<PageHeader
				title="Fournisseurs"
				description="Les clés d'API des fournisseurs de modèles utilisées par tous les runs de l'organisation."
			/>
			<ComingSoon
				icon={KeyRound}
				title="Brancher vos fournisseurs"
				summary="Vous pourrez enregistrer une clé Anthropic, Mistral ou un endpoint compatible OpenAI (Ollama, vLLM), puis la tester avant de l'utiliser."
				points={[
					'Une clé par fournisseur, partagée par l’organisation.',
					'La clé n’est jamais réaffichée : seul un aperçu de ses 4 derniers caractères reste visible.',
					'Réservé aux administrateurs ; les membres voient quels fournisseurs sont configurés.',
				]}
			/>
		</>
	)
}
