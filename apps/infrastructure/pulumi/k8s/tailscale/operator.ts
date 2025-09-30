import * as k8s from '@pulumi/kubernetes';
import { namespace } from './namespace';

new k8s.helm.v4.Chart('tailscale-operator', {
  chart: 'tailscale-operator',
  namespace: namespace.metadata.name,
  repositoryOpts: {
    repo: 'https://pkgs.tailscale.com/helmcharts',
  },

  values: {
    operatorConfig: {
      hostname: 'k8s-operator',
      nodeSelector: { 'node-role.kubernetes.io/control-plane': '' },
      tolerations: [{ key: 'node-role.kubernetes.io/control-plane', operator: 'Exists' }],
    },

    apiServerProxyConfig: {
      allowImpersonation: 'true',
    },
  },
});
