import { Plus } from 'lucide-react'
import { SectionCard } from '#/components/layout/section-card'
import { Button } from '#/components/ui/button'
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from '#/components/ui/table'
import type { ApiKey } from '#/domain/agent-team/agent-config'

interface Props {
	keys: ReadonlyArray<ApiKey>
	onCreate: () => void
	onRevoke: (index: number) => void
}

export function KeysTab({ keys, onCreate, onRevoke }: Props) {
	return (
		<SectionCard
			title="Clés API"
			bare
			actions={
				<Button type="button" size="sm" onClick={onCreate}>
					<Plus aria-hidden="true" />
					Créer une clé
				</Button>
			}
		>
			<div className="overflow-hidden rounded-lg border bg-card">
				<Table>
					<TableHeader>
						<TableRow>
							<TableHead>Nom</TableHead>
							<TableHead>Clé</TableHead>
							<TableHead>Créée</TableHead>
							<TableHead>Dernier usage</TableHead>
							<TableHead>
								<span className="sr-only">Actions</span>
							</TableHead>
						</TableRow>
					</TableHeader>
					<TableBody>
						{keys.length === 0 ? (
							<TableRow>
								<TableCell
									colSpan={5}
									className="text-center text-muted-foreground"
								>
									Aucune clé active.
								</TableCell>
							</TableRow>
						) : (
							keys.map((key, index) => (
								<TableRow key={`${key.name}-${key.preview}`}>
									<TableCell>{key.name}</TableCell>
									<TableCell className="font-mono">{key.preview}</TableCell>
									<TableCell className="text-muted-foreground">
										{key.created}
									</TableCell>
									<TableCell className="text-muted-foreground">
										{key.lastUsed}
									</TableCell>
									<TableCell className="text-right">
										<Button
											type="button"
											variant="outline"
											size="sm"
											aria-label={`Révoquer ${key.name}`}
											onClick={() => onRevoke(index)}
										>
											Révoquer
										</Button>
									</TableCell>
								</TableRow>
							))
						)}
					</TableBody>
				</Table>
			</div>
		</SectionCard>
	)
}
