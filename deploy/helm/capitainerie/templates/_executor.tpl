{{- define "capitainerie.egressProxyName" -}}
{{- include "capitainerie.componentName" (dict "root" . "component" "egress-proxy") -}}
{{- end -}}

{{- define "capitainerie.egressProxyUrl" -}}
{{- printf "http://%s.%s.svc:%d" (include "capitainerie.egressProxyName" .) .Release.Namespace (int .Values.egressProxy.service.port) -}}
{{- end -}}
