import * as k8s from '@pulumi/kubernetes';
import { provider } from '$k8s-baremetal/provider';

new k8s.helm.v4.Chart(
  'metrics-server@baremetal',
  {
    name: 'metrics-server',
    chart: 'metrics-server',
    namespace: 'kube-system',
    repositoryOpts: {
      repo: 'https://kubernetes-sigs.github.io/metrics-server',
    },

    values: {
      args: ['--kubelet-insecure-tls'],
    },
  },
  { provider },
);
