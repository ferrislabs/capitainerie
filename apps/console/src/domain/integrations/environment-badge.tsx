import type { Schemas } from '#/api/api.client'
import { Badge } from '#/components/ui/badge'
import {
	ENVIRONMENT_LABELS,
	ENVIRONMENT_VARIANTS,
} from '#/domain/clients/integration-labels'

interface Props {
	environment: Schemas.IntegrationEnvironmentDto
}

export function EnvironmentBadge({ environment }: Props) {
	return (
		<Badge variant={ENVIRONMENT_VARIANTS[environment]}>
			{ENVIRONMENT_LABELS[environment]}
		</Badge>
	)
}
