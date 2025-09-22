import * as k8s from '@pulumi/kubernetes';
import { provider } from '$k8s-bm/provider';
import { namespace } from './namespace';

export const chart = new k8s.helm.v4.Chart(
  'tailscale-operator@bm',
  {
    name: 'tailscale-operator',

    chart: 'tailscale-operator',
    namespace: namespace.metadata.name,
    repositoryOpts: {
      repo: 'https://pkgs.tailscale.com/helmcharts',
    },
  },
  { provider },
);
