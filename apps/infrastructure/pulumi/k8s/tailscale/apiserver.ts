import * as k8s from '@pulumi/kubernetes';
import { classes } from './proxy';

new k8s.apiextensions.CustomResource('k8s-apiserver', {
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
});
