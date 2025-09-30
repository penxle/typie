import * as k8s from '@pulumi/kubernetes';
import { issuers } from '$k8s/kube-system/cert-manager';
import { namespace } from './namespace';

const httpGateway = new k8s.apiextensions.CustomResource('http', {
  apiVersion: 'gateway.networking.k8s.io/v1',
  kind: 'Gateway',
  metadata: {
    name: 'http',
    namespace: namespace.metadata.name,
    annotations: {
      'cert-manager.io/cluster-issuer': issuers.letsencrypt.metadata.name,
    },
  },
  spec: {
    gatewayClassName: 'cilium',
    infrastructure: {
      labels: {
        'cilium.typie.io/advertise-bgp': 'true',
      },
    },
    addresses: [
      {
        type: 'IPAddress',
        value: '115.68.42.155',
      },
    ],
    listeners: [
      {
        name: 'http',
        protocol: 'HTTP',
        port: 80,
        allowedRoutes: {
          namespaces: {
            from: 'All',
          },
        },
      },
      {
        name: 'typie-co-apex-https',
        protocol: 'HTTPS',
        port: 443,
        hostname: 'typie.co',
        tls: {
          mode: 'Terminate',
          certificateRefs: [{ name: 'typie-co-apex-tls' }],
        },
        allowedRoutes: {
          namespaces: {
            from: 'All',
          },
        },
      },
      {
        name: 'typie-co-wildcard-https',
        protocol: 'HTTPS',
        port: 443,
        hostname: '*.typie.co',
        tls: {
          mode: 'Terminate',
          certificateRefs: [{ name: 'typie-co-wildcard-tls' }],
        },
        allowedRoutes: {
          namespaces: {
            from: 'All',
          },
        },
      },
      {
        name: 'typie-dev-apex-https',
        protocol: 'HTTPS',
        port: 443,
        hostname: 'typie.dev',
        tls: {
          mode: 'Terminate',
          certificateRefs: [{ name: 'typie-dev-apex-tls' }],
        },
        allowedRoutes: {
          namespaces: {
            from: 'All',
          },
        },
      },
      {
        name: 'typie-dev-wildcard-https',
        protocol: 'HTTPS',
        port: 443,
        hostname: '*.typie.dev',
        tls: {
          mode: 'Terminate',
          certificateRefs: [{ name: 'typie-dev-wildcard-tls' }],
        },
        allowedRoutes: {
          namespaces: {
            from: 'All',
          },
        },
      },
      {
        name: 'typie-me-apex-https',
        protocol: 'HTTPS',
        port: 443,
        hostname: 'typie.me',
        tls: {
          mode: 'Terminate',
          certificateRefs: [{ name: 'typie-me-apex-tls' }],
        },
        allowedRoutes: {
          namespaces: {
            from: 'All',
          },
        },
      },
      {
        name: 'typie-me-wildcard-https',
        protocol: 'HTTPS',
        port: 443,
        hostname: '*.typie.me',
        tls: {
          mode: 'Terminate',
          certificateRefs: [{ name: 'typie-me-wildcard-tls' }],
        },
        allowedRoutes: {
          namespaces: {
            from: 'All',
          },
        },
      },
      {
        name: 'typie-app-apex-https',
        protocol: 'HTTPS',
        port: 443,
        hostname: 'typie.app',
        tls: {
          mode: 'Terminate',
          certificateRefs: [{ name: 'typie-app-apex-tls' }],
        },
        allowedRoutes: {
          namespaces: {
            from: 'All',
          },
        },
      },
      {
        name: 'typie-app-wildcard-https',
        protocol: 'HTTPS',
        port: 443,
        hostname: '*.typie.app',
        tls: {
          mode: 'Terminate',
          certificateRefs: [{ name: 'typie-app-wildcard-tls' }],
        },
        allowedRoutes: {
          namespaces: {
            from: 'All',
          },
        },
      },
      {
        name: 'typie-net-apex-https',
        protocol: 'HTTPS',
        port: 443,
        hostname: 'typie.net',
        tls: {
          mode: 'Terminate',
          certificateRefs: [{ name: 'typie-net-apex-tls' }],
        },
        allowedRoutes: {
          namespaces: {
            from: 'All',
          },
        },
      },
      {
        name: 'typie-net-wildcard-https',
        protocol: 'HTTPS',
        port: 443,
        hostname: '*.typie.net',
        tls: {
          mode: 'Terminate',
          certificateRefs: [{ name: 'typie-net-wildcard-tls' }],
        },
        allowedRoutes: {
          namespaces: {
            from: 'All',
          },
        },
      },
      {
        name: 'typie-io-apex-https',
        protocol: 'HTTPS',
        port: 443,
        hostname: 'typie.io',
        tls: {
          mode: 'Terminate',
          certificateRefs: [{ name: 'typie-io-apex-tls' }],
        },
        allowedRoutes: {
          namespaces: {
            from: 'All',
          },
        },
      },
      {
        name: 'typie-io-wildcard-https',
        protocol: 'HTTPS',
        port: 443,
        hostname: '*.typie.io',
        tls: {
          mode: 'Terminate',
          certificateRefs: [{ name: 'typie-io-wildcard-tls' }],
        },
        allowedRoutes: {
          namespaces: {
            from: 'All',
          },
        },
      },
    ],
  },
});

new k8s.apiextensions.CustomResource('http-redirect', {
  apiVersion: 'gateway.networking.k8s.io/v1',
  kind: 'HTTPRoute',
  metadata: {
    name: 'http-redirect',
    namespace: namespace.metadata.name,
  },
  spec: {
    parentRefs: [
      {
        name: httpGateway.metadata.name,
        namespace: httpGateway.metadata.namespace,
        sectionName: 'http',
      },
    ],
    rules: [
      {
        filters: [
          {
            type: 'RequestRedirect',
            requestRedirect: {
              scheme: 'https',
              statusCode: 301,
            },
          },
        ],
      },
    ],
  },
});
