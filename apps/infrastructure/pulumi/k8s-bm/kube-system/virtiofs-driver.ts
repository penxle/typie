import * as k8s from '@pulumi/kubernetes';
import dedent from 'dedent';
import { provider } from '$k8s-bm/provider';

new k8s.apps.v1.DaemonSet(
  'virtiofs-driver@bm',
  {
    metadata: {
      name: 'virtiofs-driver',
      namespace: 'kube-system',
      labels: {
        app: 'virtiofs-driver',
      },
    },
    spec: {
      selector: {
        matchLabels: {
          app: 'virtiofs-driver',
        },
      },
      template: {
        metadata: {
          labels: {
            app: 'virtiofs-driver',
          },
        },
        spec: {
          nodeSelector: {
            'volumes.typie.io/attach': 'true',
          },
          hostPID: true,
          containers: [
            {
              name: 'virtiofs-driver',
              image: 'alpine:latest',
              command: [
                'sh',
                '-c',
                dedent`
                  apk add --no-cache util-linux
                  mkdir -p /var/mnt/data /host/var/mnt/data
                  mount -N1 -t virtiofs com.apple.virtio-fs.automount /var/mnt/data
                  while true; do sleep 3600; done
                `,
              ],
              resources: {
                requests: {
                  cpu: '10m',
                  memory: '32Mi',
                },
                limits: {
                  cpu: '100m',
                  memory: '64Mi',
                },
              },
              securityContext: {
                privileged: true,
              },
              volumeMounts: [
                {
                  name: 'host',
                  mountPath: '/host',
                },
              ],
            },
          ],
          volumes: [
            {
              name: 'host',
              hostPath: {
                path: '/',
              },
            },
          ],
        },
      },
    },
  },
  { provider },
);
