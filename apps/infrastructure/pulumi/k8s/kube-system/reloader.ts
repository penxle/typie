import * as k8s from '@pulumi/kubernetes';

new k8s.helm.v4.Chart('reloader', {
  chart: 'reloader',
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
