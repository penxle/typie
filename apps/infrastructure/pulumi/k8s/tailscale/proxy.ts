import * as k8s from '@pulumi/kubernetes';

// const proxyClass = new k8s.apiextensions.CustomResource(
//   'static-endpoints',
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
// );

// new k8s.apiextensions.CustomResource(
//   'ingress',
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
// );

const controlplane = new k8s.apiextensions.CustomResource('controlplane', {
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
});

export const classes = {
  controlplane,
};
