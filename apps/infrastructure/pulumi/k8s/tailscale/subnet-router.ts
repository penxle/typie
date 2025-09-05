import * as k8s from '@pulumi/kubernetes';
import { cluster } from '$aws/eks';
import { namespace } from './namespace';

new k8s.apiextensions.CustomResource('eks-router', {
  apiVersion: 'tailscale.com/v1alpha1',
  kind: 'Connector',

  metadata: {
    name: 'eks-router',
    namespace: namespace.metadata.name,
  },

  spec: {
    hostname: 'eks-router',
    subnetRouter: {
      advertiseRoutes: [cluster.kubernetesNetworkConfig.serviceIpv4Cidr],
    },
  },
});
