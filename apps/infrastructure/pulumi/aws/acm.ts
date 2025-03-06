import * as aws from '@pulumi/aws';
import * as pulumi from '@pulumi/pulumi';
import { zones } from '$aws/route53';

const createCertificate = (zoneId: pulumi.Input<string>, domain: string) => {
  const certificate = new aws.acm.Certificate(domain, {
    domainName: domain,
    subjectAlternativeNames: [`*.${domain}`],
    validationMethod: 'DNS',
  });

  certificate.domainValidationOptions.apply((options) => {
    for (const option of options) {
      if (option.domainName !== domain) {
        continue;
      }

      const name = option.resourceRecordName.slice(0, -1);

      new aws.route53.Record(name, {
        zoneId,
        type: option.resourceRecordType,
        name,
        records: [option.resourceRecordValue.slice(0, -1)],
        ttl: 300,
      });
    }
  });

  new aws.acm.CertificateValidation(domain, {
    certificateArn: certificate.arn,
  });

  return certificate;
};

export const certificates = {
  glitter_pizza: createCertificate(zones.glitter_pizza.zoneId, 'glitter.pizza'),
  glitter_rocks: createCertificate(zones.glitter_rocks.zoneId, 'glitter.rocks'),
};
