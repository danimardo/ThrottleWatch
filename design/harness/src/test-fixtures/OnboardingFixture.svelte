<script lang="ts">
  import { OnboardingFlow, type OnboardingStepContent, type OnboardingDetectionContent } from '../../../components';

  const finalStep: OnboardingDetectionContent = { title: 'Paso cinco', body: 'Quinto', status: 'complete' };
  const steps: [OnboardingStepContent, OnboardingStepContent, OnboardingStepContent, OnboardingStepContent, OnboardingDetectionContent] = [
    { title: 'Paso uno', body: 'Primero' },
    { title: 'Paso dos', body: 'Segundo' },
    { title: 'Paso tres', body: 'Tercero' },
    { title: 'Paso cuatro', body: 'Cuarto' },
    finalStep
  ];
  let finished = $state(false);
</script>

<OnboardingFlow
  {steps}
  backLabel="Atrás"
  nextLabel="Siguiente"
  finishLabel="Terminar"
  onFinish={() => (finished = true)}
  />
<output data-testid="finished">{finished ? 'sí' : 'no'}</output>
