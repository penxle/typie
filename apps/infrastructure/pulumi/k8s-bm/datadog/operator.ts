import * as k8s from '@pulumi/kubernetes';
import { provider } from '$k8s-bm/provider';
import { namespace } from './namespace';

new k8s.helm.v4.Chart(
  'datadog-operator@bm',
  {
    name: 'datadog-operator',

    chart: 'datadog-operator',
    namespace: namespace.metadata.name,
    repositoryOpts: {
      repo: 'https://helm.datadoghq.com',
    },
  },
  { provider },
);
