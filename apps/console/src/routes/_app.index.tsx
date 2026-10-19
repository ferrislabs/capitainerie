import { createFileRoute } from '@tanstack/react-router'
import { HomePage } from '#/domain/home/feature/home-page'

export const Route = createFileRoute('/_app/')({ component: HomePage })
