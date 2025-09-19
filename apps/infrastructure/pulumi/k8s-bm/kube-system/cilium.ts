import * as k8s from '@pulumi/kubernetes';
import { provider } from '$k8s-bm/provider';

new k8s.helm.v4.Chart(
  'cilium@bm',
  {
    name: 'cilium',

    chart: 'cilium',
    namespace: 'kube-system',
    repositoryOpts: {
      repo: 'https://helm.cilium.io/',
    },

    values: {
      k8sServiceHost: 'localhost',
      k8sServicePort: 7445,

      kubeProxyReplacement: true,

      ipam: {
        mode: 'cluster-pool',
        operator: {
          clusterPoolIPv4PodCIDRList: '10.10.0.0/16',
          clusterPoolIPv4MaskSize: 24,
        },
      },

      bgpControlPlane: {
        enabled: true,
      },

      ingressController: {
        enabled: true,
        loadBalancerMode: 'shared',

        service: {
          type: 'NodePort',
          insecureNodePort: 32_080,
          secureNodePort: 32_443,
        },
      },

      // spell-checker:disable
      securityContext: {
        capabilities: {
          ciliumAgent: [
            'CHOWN',
            'KILL',
            'NET_ADMIN',
            'NET_RAW',
            'IPC_LOCK',
            'SYS_ADMIN',
            'SYS_RESOURCE',
            'DAC_OVERRIDE',
            'FOWNER',
            'SETGID',
            'SETUID',
          ],
          cleanCiliumState: ['NET_ADMIN', 'SYS_ADMIN', 'SYS_RESOURCE'],
        },
      },
      // spell-checker:enable

      hubble: {
        relay: {
          enabled: true,
        },

        ui: {
          enabled: true,
        },
      },
    },
  },
  { provider },
);

new k8s.apiextensions.CustomResource(
  'default@bm',
  {
    apiVersion: 'cilium.io/v2',
    kind: 'CiliumLoadBalancerIPPool',
    metadata: {
      name: 'default',
    },
    spec: {
      allowFirstLastIPs: 'No',
      blocks: [{ cidr: '10.30.0.0/16' }],
    },
  },
  { provider },
);

new k8s.apiextensions.CustomResource(
  'default@bm',
  {
    apiVersion: 'cilium.io/v2',
    kind: 'CiliumBGPClusterConfig',

    metadata: {
      name: 'default',
    },

    spec: {
      bgpInstances: [
        {
          name: '65001',
          localASN: 65_001,
          peers: [
            {
              name: '65000',
              peerASN: 65_000,
              autoDiscovery: {
                mode: 'DefaultGateway',
                defaultGateway: {
                  addressFamily: 'ipv4',
                },
              },
              peerConfigRef: {
                name: 'default',
              },
            },
          ],
        },
      ],
    },
  },
  { provider },
);

new k8s.apiextensions.CustomResource(
  'default@bm',
  {
    apiVersion: 'cilium.io/v2',
    kind: 'CiliumBGPPeerConfig',

    metadata: {
      name: 'default',
    },

    spec: {
      gracefulRestart: {
        enabled: true,
      },
    },
  },
  { provider },
);

new k8s.apiextensions.CustomResource(
  'default@bm',
  {
    apiVersion: 'cilium.io/v2',
    kind: 'CiliumBGPAdvertisement',

    metadata: {
      name: 'default',
    },

    spec: {
      advertisements: [
        {
          advertisementType: 'Service',

          service: {
            addresses: ['LoadBalancerIP'],
          },

          attributes: {
            communities: {
              standard: ['65000:100'],
            },
          },
        },
      ],
    },
  },
  { provider },
);
