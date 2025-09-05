import * as k8s from '@pulumi/kubernetes';
import { namespace } from './namespace';

new k8s.helm.v4.Chart('tailscale-operator', {
  chart: 'tailscale-operator',
  namespace: namespace.metadata.name,
  repositoryOpts: {
    repo: 'https://pkgs.tailscale.com/helmcharts',
  },
});
