import * as k8s from '@pulumi/kubernetes';

new k8s.helm.v4.Chart('external-secrets', {
  chart: 'external-secrets',
  namespace: 'kube-system',
  repositoryOpts: {
    repo: 'https://charts.external-secrets.io',
  },
});
