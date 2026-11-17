import { Badge } from '#/components/ui/badge'
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from '#/components/ui/table'
import type { Artifact } from '#/domain/artifacts/mock/artifacts'
import { shortName } from '#/domain/demo'

interface Props {
	artifacts: ReadonlyArray<Artifact>
}

export function ArtifactsTable({ artifacts }: Props) {
	return (
		<div className="overflow-hidden rounded-lg border bg-card">
			<Table>
				<TableHeader>
					<TableRow>
						<TableHead>Nom</TableHead>
						<TableHead>Produit par</TableHead>
						<TableHead>Type</TableHead>
						<TableHead>Mis à jour</TableHead>
					</TableRow>
				</TableHeader>
				<TableBody>
					{artifacts.map((artifact) => (
						<TableRow key={artifact.id}>
							<TableCell className="font-medium">{artifact.name}</TableCell>
							<TableCell className="text-muted-foreground">
								{shortName(artifact.agent)}
							</TableCell>
							<TableCell>
								<Badge variant="neutral">{artifact.type}</Badge>
							</TableCell>
							<TableCell className="text-muted-foreground">
								{artifact.updated}
							</TableCell>
						</TableRow>
					))}
				</TableBody>
			</Table>
		</div>
	)
}
