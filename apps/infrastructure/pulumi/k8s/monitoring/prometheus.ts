import * as k8s from '@pulumi/kubernetes';
import * as random from '@pulumi/random';
import { namespace } from './namespace';

const grafanaRole = new k8s.rbac.v1.Role('prometheus-grafana', {
  metadata: {
    name: 'prometheus-grafana',
    namespace: namespace.metadata.name,
  },
});

const grafanaPassword = new random.RandomPassword('admin@grafana', {
  length: 20,
});

const grafanaAdminSecret = new k8s.core.v1.Secret('grafana-admin', {
  metadata: {
    name: 'grafana-admin',
    namespace: namespace.metadata.name,
  },
  stringData: {
    username: 'admin',
    password: grafanaPassword.result,
  },
});

new k8s.helm.v4.Chart('prometheus', {
  chart: 'kube-prometheus-stack',
  namespace: namespace.metadata.name,
  repositoryOpts: {
    repo: 'https://prometheus-community.github.io/helm-charts',
  },

  values: {
    nameOverride: 'prometheus',
    fullnameOverride: 'prometheus',
    cleanPrometheusOperatorObjectNames: true,

    kubeControllerManager: { enabled: false },
    kubeEtcd: { enabled: false },
    kubeScheduler: { enabled: false },

    prometheus: {
      prometheusSpec: {
        ruleSelectorNilUsesHelmValues: false,
        serviceMonitorSelectorNilUsesHelmValues: false,
        podMonitorSelectorNilUsesHelmValues: false,
        probeSelectorNilUsesHelmValues: false,
        scrapeConfigSelectorNilUsesHelmValues: false,

        retention: '180d',
        retentionSize: '200GB',

        storageSpec: {
          volumeClaimTemplate: {
            spec: {
              storageClassName: 'gp3',
              accessModes: ['ReadWriteOnce'],
              resources: {
                requests: {
                  storage: '200Gi',
                },
              },
            },
          },
        },
      },
    },

    alertmanager: {
      alertmanagerSpec: {
        storage: {
          volumeClaimTemplate: {
            spec: {
              storageClassName: 'gp3',
              accessModes: ['ReadWriteOnce'],
              resources: {
                requests: {
                  storage: '20Gi',
                },
              },
            },
          },
        },
      },
    },

    prometheusOperator: {
      admissionWebhooks: {
        certManager: {
          enabled: true,
        },
      },
    },

    'kube-state-metrics': {
      metricLabelsAllowlist: ['nodes=[topology.kubernetes.io/zone]'],
    },

    grafana: {
      'grafana.ini': {
        server: {
          root_url: 'https://grafana.typie.io',
        },
      },

      imageRenderer: {
        enabled: true,
      },

      admin: {
        existingSecret: grafanaAdminSecret.metadata.name,
        userKey: 'username',
        passwordKey: 'password',
      },

      rbac: {
        useExistingRole: grafanaRole.metadata.name,
      },

      service: {
        type: 'NodePort',
      },

      ingress: {
        enabled: true,
        ingressClassName: 'alb',

        annotations: {
          'alb.ingress.kubernetes.io/group.name': 'private-alb',
          'alb.ingress.kubernetes.io/listen-ports': JSON.stringify([{ HTTPS: 443 }]),
          'alb.ingress.kubernetes.io/healthcheck-path': '/healthz',
        },

        hosts: ['grafana.typie.io'],
      },
    },
  },
});

export const outputs = {
  K8S_MONITORING_GRAFANA_PASSWORD: grafanaPassword.result,
};
