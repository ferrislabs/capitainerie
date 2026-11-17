import { Plus } from 'lucide-react'
import { SectionCard } from '#/components/layout/section-card'
import { Button } from '#/components/ui/button'
import type { ApiKey } from '#/domain/agent-team/agent-config'
import { Tbl, Td, Tr } from '#/domain/agent-team/ui/tbl'

interface Props {
	keys: ReadonlyArray<ApiKey>
	onCreate: () => void
	onRevoke: (index: number) => void
}

export function KeysTab({ keys, onCreate, onRevoke }: Props) {
	return (
		<SectionCard
			title="Clés API"
			flush
			actions={
				<Button type="button" size="sm" onClick={onCreate}>
					<Plus aria-hidden="true" />
					Créer une clé
				</Button>
			}
		>
			<Tbl head={['Nom', 'Clé', 'Créée', 'Dernier usage', '']}>
				{keys.length === 0 ? (
					<Tr>
						<Td colSpan={5} className="text-center text-muted-foreground">
							Aucune clé active.
						</Td>
					</Tr>
				) : (
					keys.map((key, index) => (
						<Tr key={`${key.name}-${key.preview}`}>
							<Td>{key.name}</Td>
							<Td className="font-mono text-[11.5px]">{key.preview}</Td>
							<Td className="text-muted-foreground">{key.created}</Td>
							<Td className="text-muted-foreground">{key.lastUsed}</Td>
							<Td className="text-right">
								<Button
									type="button"
									variant="outline"
									size="sm"
									aria-label={`Révoquer ${key.name}`}
									onClick={() => onRevoke(index)}
								>
									Révoquer
								</Button>
							</Td>
						</Tr>
					))
				)}
			</Tbl>
		</SectionCard>
	)
}
