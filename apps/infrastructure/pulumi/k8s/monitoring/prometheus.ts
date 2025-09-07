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
      config: {
        templates: ['/etc/alertmanager/configmaps/alertmanager-templates/*.tmpl'],
      },

      alertmanagerSpec: {
        configMaps: ['alertmanager-templates'],

        alertmanagerConfigMatcherStrategy: {
          type: 'None',
        },

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

    defaultRules: {
      disabled: {
        KubeCPUOvercommit: true,
        KubeMemoryOvercommit: true,
        NodeMemoryMajorPagesFaults: true,
      },
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

new k8s.apiextensions.CustomResource('alertmanager', {
  apiVersion: 'monitoring.coreos.com/v1alpha1',
  kind: 'AlertmanagerConfig',

  metadata: {
    name: 'alertmanager',
    namespace: namespace.metadata.name,
  },

  spec: {
    route: {
      receiver: 'slack',

      groupBy: ['namespace', 'alertname'],
      matchers: [
        { matchType: '!=', name: 'alertname', value: 'Watchdog' },
        { matchType: '!=', name: 'alertname', value: 'InfoInhibitor' },
      ],

      groupWait: '10s',
      groupInterval: '5m',
      repeatInterval: '1h',
    },

    receivers: [
      {
        name: 'slack',
        slackConfigs: [
          {
            channel: '#monitoring',
            sendResolved: true,

            apiURL: {
              name: 'alertmanager-secrets',
              key: 'slack-api-url',
            },

            httpConfig: {
              authorization: {
                type: 'Bearer',
                credentials: {
                  name: 'alertmanager-secrets',
                  key: 'slack-bot-token',
                },
              },
            },

            title: '{{ template "slack.monzo.title" . }}',
            color: '{{ template "slack.monzo.color" . }}',
            text: '{{ template "slack.monzo.text" . }}',

            actions: [
              {
                type: 'button',
                name: 'alert',
                value: 'text',
                text: 'Alert :bell:',
                url: 'https://grafana.typie.io/alerting/list?search={{ .CommonLabels.alertname | urlquery -}}',
              },
              {
                type: 'button',
                name: 'runbook',
                value: 'text',
                text: 'Runbook :green_book:',
                url: '{{ (index .Alerts 0).Annotations.runbook_url }}',
              },
            ],
          },
        ],
      },
    ],
  },
});

export const outputs = {
  K8S_MONITORING_GRAFANA_PASSWORD: grafanaPassword.result,
};
