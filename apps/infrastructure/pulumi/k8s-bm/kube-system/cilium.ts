import * as k8s from '@pulumi/kubernetes';
import { provider } from '$k8s-bm/provider';

const chart = new k8s.helm.v4.Chart(
  'cilium@bm',
  {
    name: 'cilium',

    chart: 'cilium',
    namespace: 'kube-system',
    repositoryOpts: {
      repo: 'https://helm.cilium.io/',
    },

    values: {
      tunnelProtocol: 'geneve',
      devices: ['enp0s1'],
      ipv4NativeRoutingCIDR: '10.0.0.0/16',

      k8sServiceHost: 'localhost',
      k8sServicePort: 7445,

      kubeProxyReplacement: true,

      bpf: {
        masquerade: true,
      },

      ipam: {
        mode: 'kubernetes',
      },

      bgpControlPlane: {
        enabled: true,
      },

      loadBalancer: {
        mode: 'hybrid',
        dsrDispatch: 'geneve',
      },

      gatewayAPI: {
        enabled: true,
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

      cgroup: {
        autoMount: {
          enabled: false,
        },

        hostRoot: '/sys/fs/cgroup',
      },

      hubble: {
        relay: {
          enabled: true,
        },

        ui: {
          enabled: true,
        },
      },

      prometheus: {
        enabled: true,
      },

      operator: {
        prometheus: {
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
      blocks: [{ start: '115.68.42.155', stop: '115.68.42.158' }],
      serviceSelector: {
        matchExpressions: [
          {
            key: 'cilium.typie.io/advertise-bgp',
            operator: 'In',
            values: ['true'],
          },
        ],
      },
    },
  },
  { provider, dependsOn: [chart] },
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
              name: '65001',
              peerASN: 65_001,
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
  { provider, dependsOn: [chart] },
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
      families: [
        {
          afi: 'ipv4',
          safi: 'unicast',

          advertisements: {
            matchLabels: {
              advertise: 'bgp',
            },
          },
        },
      ],

      gracefulRestart: {
        enabled: true,
      },
    },
  },
  { provider, dependsOn: [chart] },
);

new k8s.apiextensions.CustomResource(
  'default@bm',
  {
    apiVersion: 'cilium.io/v2',
    kind: 'CiliumBGPAdvertisement',

    metadata: {
      name: 'default',
      labels: {
        advertise: 'bgp',
      },
    },

    spec: {
      advertisements: [
        {
          advertisementType: 'Service',

          service: {
            addresses: ['LoadBalancerIP'],
          },

          selector: {
            matchExpressions: [
              {
                key: 'cilium.typie.io/advertise-bgp',
                operator: 'In',
                values: ['true'],
              },
            ],
          },

          attributes: {
            communities: {
              standard: ['65001:100'],
            },
          },
        },
      ],
    },
  },
  { provider, dependsOn: [chart] },
);
