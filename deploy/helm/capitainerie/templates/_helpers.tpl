{{- define "capitainerie.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "capitainerie.fullname" -}}
{{- if .Values.fullnameOverride -}}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" -}}
{{- else -}}
{{- $name := default .Chart.Name .Values.nameOverride -}}
{{- if contains $name .Release.Name -}}
{{- .Release.Name | trunc 63 | trimSuffix "-" -}}
{{- else -}}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" -}}
{{- end -}}
{{- end -}}
{{- end -}}

{{- define "capitainerie.componentName" -}}
{{- printf "%s-%s" (include "capitainerie.fullname" .root) .component | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "capitainerie.selectorLabels" -}}
app.kubernetes.io/name: {{ include "capitainerie.name" .root }}
app.kubernetes.io/instance: {{ .root.Release.Name }}
app.kubernetes.io/component: {{ .component }}
{{- end -}}

{{- define "capitainerie.labels" -}}
helm.sh/chart: {{ printf "%s-%s" .root.Chart.Name .root.Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{ include "capitainerie.selectorLabels" . }}
app.kubernetes.io/version: {{ .root.Chart.AppVersion | quote }}
app.kubernetes.io/managed-by: {{ .root.Release.Service }}
app.kubernetes.io/part-of: capitainerie
{{- with .root.Values.global.labels }}
{{ toYaml . }}
{{- end }}
{{- end -}}

{{- define "capitainerie.image" -}}
{{- $repo := default (printf "%s-%s" .root.Values.image.prefix .component) .values.image.repository -}}
{{- printf "%s:%s" $repo (default .root.Chart.AppVersion .values.image.tag) -}}
{{- end -}}

{{- define "capitainerie.secretEnv" -}}
{{- $root := .root -}}
{{- range .keys }}
- name: {{ . }}
  valueFrom:
    secretKeyRef:
      name: {{ $root.Values.secrets.existingSecret | quote }}
      key: {{ . | quote }}
      {{- if has . $root.Values.secrets.optionalKeys }}
      optional: true
      {{- end }}
{{- end }}
{{- end -}}

{{- define "capitainerie.imagePullSecrets" -}}
{{- with .Values.global.imagePullSecrets }}
imagePullSecrets:
  {{- toYaml . | nindent 2 }}
{{- end }}
{{- end -}}

{{- define "capitainerie.topologySpread" -}}
{{- if .root.Values.topologySpread.enabled }}
topologySpreadConstraints:
  - maxSkew: 1
    topologyKey: kubernetes.io/hostname
    whenUnsatisfiable: ScheduleAnyway
    labelSelector:
      matchLabels:
        {{- include "capitainerie.selectorLabels" . | nindent 8 }}
{{- end }}
{{- end -}}

{{- define "capitainerie.scheduling" -}}
{{- with .nodeSelector }}
nodeSelector:
  {{- toYaml . | nindent 2 }}
{{- end }}
{{- with .tolerations }}
tolerations:
  {{- toYaml . | nindent 2 }}
{{- end }}
{{- with .affinity }}
affinity:
  {{- toYaml . | nindent 2 }}
{{- end }}
{{- end -}}

{{- define "capitainerie.mcpUrl" -}}
{{- if .Values.worker.mcpUrl -}}
{{- .Values.worker.mcpUrl -}}
{{- else -}}
{{- printf "http://%s.%s.svc:%d/mcp" (include "capitainerie.componentName" (dict "root" . "component" "mcp-kanban")) .Release.Namespace (int .Values.mcpKanban.service.port) -}}
{{- end -}}
{{- end -}}

{{- define "capitainerie.egressDependency" -}}
- ports:
    {{- range .ports }}
    - protocol: TCP
      port: {{ . }}
    {{- end }}
  {{- with .to }}
  to:
    {{- toYaml . | nindent 4 }}
  {{- end }}
{{- end -}}

{{- define "capitainerie.egressDns" -}}
- to:
    {{- toYaml .Values.networkPolicy.dnsTo | nindent 4 }}
  ports:
    - protocol: UDP
      port: 53
    - protocol: TCP
      port: 53
{{- end -}}
