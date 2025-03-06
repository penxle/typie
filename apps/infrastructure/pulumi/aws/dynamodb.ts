import * as aws from '@pulumi/aws';

const externalDns = new aws.dynamodb.Table('external-dns', {
  name: 'external-dns',
  billingMode: 'PAY_PER_REQUEST',

  hashKey: 'k',
  attributes: [{ name: 'k', type: 'S' }],
});

export const tables = {
  externalDns,
};
