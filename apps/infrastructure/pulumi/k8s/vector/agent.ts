import * as k8s from '@pulumi/kubernetes';
import { namespace } from './namespace';

new k8s.helm.v4.Chart('vector-agent', {
  chart: 'vector',
  namespace: namespace.metadata.name,
  repositoryOpts: {
    repo: 'https://helm.vector.dev',
  },
  values: {
    role: 'Agent',
    tolerations: [{ key: 'CriticalAddonsOnly', operator: 'Exists' }],
    customConfig: {
      data_dir: '/vector-data-dir',
      sources: {
        kubernetes_logs: {
          type: 'kubernetes_logs',
        },
      },
      sinks: {
        vector: {
          inputs: ['*'],
          type: 'vector',
          address: 'vector-aggregator:6000',
        },
      },
    },
  },
});
