import * as pulumi from '@pulumi/pulumi';
import * as typie from '@typie/pulumi';

const config = new pulumi.Config('typie');

new typie.Service('website', {
  name: 'website',

  image: {
    name: '509399603331.dkr.ecr.ap-northeast-2.amazonaws.com/typie',
    digest: config.require('digest'),
    command: ['bun', 'run', 'apps/website/index.js'],
  },

  resources: {
    cpu: '500m',
    memory: '1Gi',
  },

  autoscale: {
    minCount: 2,
    maxCount: 20,
    averageCpuUtilization: 50,
  },

  secret: {
    project: 'typie-website',
  },

  ingress: {
    domain: {
      production: 'typie.co',
      dev: 'typie.dev',
    },

    priority: {
      production: '21',
      dev: '121',
    },

    cloudfront: {
      production: {
        domainZone: 'typie.co',
      },
    },
  },
});

new typie.Redirect('www.website', {
  name: 'www.website',

  priority: {
    production: '22',
    dev: '122',
  },

  production: {
    from: { host: 'www.typie.co' },
    to: { host: 'typie.co' },
  },

  dev: {
    from: { host: 'www.typie.dev' },
    to: { host: 'typie.dev' },
  },

  code: 301,
});
