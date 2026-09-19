import type { AnalysisEvidence } from '../../design-system/components/AnalysisScreen.svelte';
import type { Classification } from '../../design-system/lib/classification';
import type { Tone } from '../../design-system/tokens/tokens';

export interface DiagnosticView {
  classification: Classification;
  coverage: 'A' | 'B' | 'C';
  confidence: number;
  severity?: 'boost' | 'below_base';
  evidence: string[];
  alternativeCauses: string[];
  analyzedFromMs?: number;
  analyzedToMs?: number;
}

export interface NarrativeView {
  evidenceLine: string;
  evidence: string[];
  alternativeCauses: string[];
  analysisEvidence: AnalysisEvidence;
  causalChain?: Array<{ id: string; label: string; value: string; tone: Tone }>;
}

const confidenceLabel = (confidence: number): string =>
  confidence >= 0.75
    ? 'confianza alta'
    : confidence >= 0.45
      ? 'confianza media'
      : 'confianza baja';

const toneFor = (classification: Classification, index: number): Tone => {
  if (index === 0) return 'accent';
  if (classification.includes('thermal')) return 'thermal';
  if (classification.includes('power')) return 'power';
  if (classification.includes('platform')) return 'warm';
  return 'accent';
};

export function toNarrativeView(diagnostic: DiagnosticView): NarrativeView {
  const interval =
    diagnostic.analyzedFromMs !== undefined &&
    diagnostic.analyzedToMs !== undefined
      ? `intervalo ${String(Math.round((diagnostic.analyzedToMs - diagnostic.analyzedFromMs) / 1000))} s`
      : 'intervalo no disponible';
  const evidenceLine = `${confidenceLabel(diagnostic.confidence)} · cobertura ${diagnostic.coverage} · ${interval}`;
  const causalChain =
    diagnostic.evidence.length >= 2 &&
    !diagnostic.evidence.includes('turbo_end')
      ? diagnostic.evidence.slice(0, 4).map((code, index) => ({
          id: `${code}-${String(index)}`,
          label: code,
          value: code,
          tone: toneFor(diagnostic.classification, index)
        }))
      : undefined;
  return {
    evidenceLine,
    evidence: diagnostic.evidence,
    alternativeCauses: diagnostic.alternativeCauses,
    analysisEvidence: {
      title: 'Evidencia del diagnóstico',
      description: evidenceLine,
      items: [
        { label: 'Clasificación', value: diagnostic.classification },
        { label: 'Cobertura', value: diagnostic.coverage },
        { label: 'Gravedad', value: diagnostic.severity ?? 'no aplicable' },
        { label: 'Confianza', value: confidenceLabel(diagnostic.confidence) }
      ]
    },
    causalChain
  };
}
