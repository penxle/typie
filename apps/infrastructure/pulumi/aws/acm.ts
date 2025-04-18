import * as aws from '@pulumi/aws';
import * as pulumi from '@pulumi/pulumi';
import { zones } from '$aws/route53';

const createCertificate = (zoneId: pulumi.Input<string>, domain: string, ...subjectAlternativeNames: string[]) => {
  const certificate = new aws.acm.Certificate(domain, {
    domainName: domain,
    subjectAlternativeNames: [`*.${domain}`, ...subjectAlternativeNames],
    validationMethod: 'DNS',
  });

  certificate.domainValidationOptions.apply((options) => {
    const resourceRecordNames = new Set<string>();

    for (const option of options) {
      if (resourceRecordNames.has(option.resourceRecordName)) {
        continue;
      }

      resourceRecordNames.add(option.resourceRecordName);

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
  typie_co: createCertificate(zones.typie_co.zoneId, 'typie.co'),
  typie_dev: createCertificate(zones.typie_dev.zoneId, 'typie.dev', '*.usersite.typie.dev'),
  typie_me: createCertificate(zones.typie_me.zoneId, 'typie.me'),
  typie_net: createCertificate(zones.typie_net.zoneId, 'typie.net'),
  typie_io: createCertificate(zones.typie_io.zoneId, 'typie.io'),
};
