import * as k8s from '@pulumi/kubernetes';

new k8s.helm.v4.Chart('argo-rollouts', {
  chart: 'argo-rollouts',
  namespace: 'kube-system',
  repositoryOpts: {
    repo: 'https://argoproj.github.io/argo-helm',
  },

  values: {
    notifications: {
      secret: {
        create: true,
      },
    },
  },
});
