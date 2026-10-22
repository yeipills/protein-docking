{{/*
Expand the name of the chart.
*/}}
{{- define "protein-docking.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
*/}}
{{- define "protein-docking.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .Chart.Name .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{/*
Create chart name and version as used by the chart label.
*/}}
{{- define "protein-docking.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "protein-docking.labels" -}}
helm.sh/chart: {{ include "protein-docking.chart" . }}
{{ include "protein-docking.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
app.kubernetes.io/part-of: protein-docking
{{- end }}

{{/*
Selector labels
*/}}
{{- define "protein-docking.selectorLabels" -}}
app.kubernetes.io/name: {{ include "protein-docking.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
Create the name of the service account to use
*/}}
{{- define "protein-docking.serviceAccountName" -}}
{{- if .Values.serviceAccount.create }}
{{- default (include "protein-docking.fullname" .) .Values.serviceAccount.name }}
{{- else }}
{{- default "default" .Values.serviceAccount.name }}
{{- end }}
{{- end }}

{{/*
Create the database URL
*/}}
{{- define "protein-docking.databaseUrl" -}}
{{- printf "postgresql://%s:%s@%s:%d/%s" .Values.postgresql.auth.username .Values.postgresql.auth.password (include "postgresql.primary.fullname" .Subcharts.postgresql) .Values.postgresql.primary.service.ports.postgresql .Values.postgresql.auth.database }}
{{- end }}

{{/*
Create Redis URLs for cluster
*/}}
{{- define "protein-docking.redisUrls" -}}
{{- $redisHost := include "redis.fullname" .Subcharts.redis -}}
{{- printf "redis://%s-master:6379,redis://%s-replica:6379" $redisHost $redisHost }}
{{- end }}