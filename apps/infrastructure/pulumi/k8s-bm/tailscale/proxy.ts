import * as k8s from '@pulumi/kubernetes';
import { provider } from '$k8s-bm/provider';

// const proxyClass = new k8s.apiextensions.CustomResource(
//   'static-endpoints@bm',
//   {
//     apiVersion: 'tailscale.com/v1alpha1',
//     kind: 'ProxyClass',

//     metadata: {
//       name: 'static-endpoints',
//     },

//     spec: {
//       statefulSet: {
//         pod: {
//           priorityClassName: 'system-node-critical',
//           nodeSelector: { 'kubernetes.io/hostname': 'capybara' },
//           tolerations: [
//             { key: 'node-role.kubernetes.io/control-plane', operator: 'Exists' },
//             { key: 'CriticalAddonsOnly', operator: 'Exists' },
//           ],
//         },
//       },

//       staticEndpoints: {
//         nodePort: {
//           ports: [{ port: 31_667, endPort: 31_668 }],
//           selector: {
//             'kubernetes.io/hostname': 'capybara',
//           },
//         },
//       },
//     },
//   },
//   { provider },
// );

// new k8s.apiextensions.CustomResource(
//   'ingress@bm',
//   {
//     apiVersion: 'tailscale.com/v1alpha1',
//     kind: 'ProxyGroup',

//     metadata: {
//       name: 'ingress',
//       namespace: namespace.metadata.name,
//     },

//     spec: {
//       type: 'ingress',
//       // proxyClass: proxyClass.metadata.name,
//       replicas: 2,
//     },
//   },
//   { provider },
// );

const controlplane = new k8s.apiextensions.CustomResource(
  'controlplane@bm',
  {
    apiVersion: 'tailscale.com/v1alpha1',
    kind: 'ProxyClass',

    metadata: {
      name: 'controlplane',
    },

    spec: {
      statefulSet: {
        pod: {
          priorityClassName: 'system-cluster-critical',
          nodeSelector: { 'node-role.kubernetes.io/control-plane': '' },
          tolerations: [{ key: 'node-role.kubernetes.io/control-plane', operator: 'Exists' }],
        },
      },
    },
  },
  { provider },
);

export const classes = {
  controlplane,
};
