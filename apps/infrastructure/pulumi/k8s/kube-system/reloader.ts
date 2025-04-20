import * as k8s from '@pulumi/kubernetes';

new k8s.helm.v4.Chart('reloader', {
  chart: 'reloader',
  version: '2.0.0',
  namespace: 'kube-system',
  repositoryOpts: {
    repo: 'https://stakater.github.io/stakater-charts',
  },

  values: {
    reloader: {
      isArgoRollouts: true,
      reloadOnCreate: true,
      syncAfterRestart: true,
    },
  },
});
