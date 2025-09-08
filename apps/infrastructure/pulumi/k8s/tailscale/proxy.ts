import * as k8s from '@pulumi/kubernetes';
import { namespace } from './namespace';

new k8s.apiextensions.CustomResource('static', {
  apiVersion: 'tailscale.com/v1alpha1',
  kind: 'ProxyClass',

  metadata: {
    name: 'static',
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

// const ingressProxyGroup = new k8s.apiextensions.CustomResource('ingress', {
//   apiVersion: 'tailscale.com/v1alpha1',
//   kind: 'ProxyGroup',

//   metadata: {
//     name: 'ingress',
//     namespace: namespace.metadata.name,
//   },

//   spec: {
//     type: 'ingress',
//     proxyClass: proxyClass.metadata.name,
//   },
// });
