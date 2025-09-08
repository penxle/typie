import * as k8s from '@pulumi/kubernetes';
import { namespace } from './namespace';

new k8s.core.v1.Pod('node-provisioner@tailscale', {
  metadata: {
    name: 'node-provisioner',
    namespace: namespace.metadata.name,
  },
  spec: {
    containers: [{ name: 'pause', image: 'registry.k8s.io/pause:3.10', command: ['./pause'] }],
    nodeSelector: { 'karpenter.sh/nodepool': 'tailnet' },
    tolerations: [{ key: 'karpenter.sh/nodepool', value: 'tailnet', effect: 'NoSchedule' }],
  },
});

const proxyClass = new k8s.apiextensions.CustomResource('static-endpoints', {
  apiVersion: 'tailscale.com/v1alpha1',
  kind: 'ProxyClass',

  metadata: {
    name: 'static-endpoints',
    namespace: namespace.metadata.name,
  },

  spec: {
    statefulSet: {
      pod: {
        nodeSelector: { 'karpenter.sh/nodepool': 'tailnet' },
        tolerations: [{ key: 'karpenter.sh/nodepool', value: 'tailnet', effect: 'NoSchedule' }],
      },
    },

    staticEndpoints: {
      nodePort: {
        ports: [{ port: 31_667, endPort: 31_680 }],
        selector: {
          'karpenter.sh/nodepool': 'tailnet',
        },
      },
    },
  },
});

new k8s.apiextensions.CustomResource('ingress', {
  apiVersion: 'tailscale.com/v1alpha1',
  kind: 'ProxyGroup',

  metadata: {
    name: 'ingress',
    namespace: namespace.metadata.name,
  },

  spec: {
    type: 'ingress',
    proxyClass: proxyClass.metadata.name,
    replicas: 2,
  },
});
