import { Notice } from '#/components/layout/notice'

const FLOATING =
	'fixed bottom-[18px] left-1/2 z-[60] -translate-x-1/2 rounded-lg border-0 bg-foreground px-3.5 py-2 font-medium text-background shadow-float'

export function Toast({ message }: { message: string | null }) {
	return <Notice message={message} className={message ? FLOATING : undefined} />
}
