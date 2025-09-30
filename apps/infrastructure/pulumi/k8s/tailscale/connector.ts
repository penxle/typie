import * as k8s from '@pulumi/kubernetes';
import { classes } from './proxy';

new k8s.apiextensions.CustomResource('k8s-subnet-router', {
  apiVersion: 'tailscale.com/v1alpha1',
  kind: 'Connector',
  metadata: {
    name: 'k8s-subnet-router',
  },
  spec: {
    hostnamePrefix: 'k8s-subnet-router',
    proxyClass: classes.controlplane.metadata.name,
    replicas: 2,
    subnetRouter: {
      advertiseRoutes: ['10.10.0.0/16', '10.20.0.0/16'],
    },
  },
});
