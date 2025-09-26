import * as k8s from '@pulumi/kubernetes';
import { provider } from '$k8s-bm/provider';
import { classes } from './proxy';

new k8s.apiextensions.CustomResource(
  'k8s-apiserver@bm',
  {
    apiVersion: 'tailscale.com/v1alpha1',
    kind: 'ProxyGroup',

    metadata: {
      name: 'k8s-apiserver',
    },

    spec: {
      type: 'kube-apiserver',
      proxyClass: classes.controlplane.metadata.name,
      replicas: 2,
      kubeAPIServer: {
        mode: 'auth',
        hostname: 'k8s',
      },
    },
  },
  { provider },
);
