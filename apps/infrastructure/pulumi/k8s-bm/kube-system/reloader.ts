import * as k8s from '@pulumi/kubernetes';
import { provider } from '$k8s-bm/provider';

new k8s.helm.v4.Chart(
  'reloader@bm',
  {
    name: 'reloader',

    chart: 'reloader',
    namespace: 'kube-system',
    repositoryOpts: {
      repo: 'https://stakater.github.io/stakater-charts',
    },

    values: {
      reloader: {
        reloadOnCreate: true,
        syncAfterRestart: true,
      },
    },
  },
  { provider },
);
