// cspell:ignore OEBPS oebps dcterms idref
import { XMLBuilder } from 'fast-xml-parser';

const xmlBuilder = new XMLBuilder({
  ignoreAttributes: false,
  attributeNamePrefix: '@_',
  format: true,
  indentBy: '  ',
  suppressEmptyNode: true,
});

const XML_DECLARATION = '<?xml version="1.0" encoding="UTF-8"?>\n';

export function generateContainerXml(): string {
  return (
    XML_DECLARATION +
    xmlBuilder.build({
      container: {
        '@_version': '1.0',
        '@_xmlns': 'urn:oasis:names:tc:opendocument:xmlns:container',
        rootfiles: {
          rootfile: {
            '@_full-path': 'OEBPS/content.opf',
            '@_media-type': 'application/oebps-package+xml',
          },
        },
      },
    })
  );
}

export function generateContentOpf(params: {
  title: string;
  author: string;
  images: { id: string; filename: string; mediaType: string }[];
  fonts: { filename: string }[];
}): string {
  const { title, author, images, fonts } = params;
  const uuid = crypto.randomUUID();
  const modified = new Date().toISOString().replace(/\.\d{3}Z$/, 'Z');

  const manifestItems = [
    { '@_id': 'nav', '@_href': 'nav.xhtml', '@_media-type': 'application/xhtml+xml', '@_properties': 'nav' },
    { '@_id': 'document', '@_href': 'document.xhtml', '@_media-type': 'application/xhtml+xml' },
    { '@_id': 'style', '@_href': 'style.css', '@_media-type': 'text/css' },
    ...images.map((img) => ({
      '@_id': `img-${img.id}`,
      '@_href': `images/${img.filename}`,
      '@_media-type': img.mediaType,
    })),
    ...fonts.map((f, i) => ({
      '@_id': `font-${i}`,
      '@_href': `fonts/${f.filename}`,
      '@_media-type': 'application/font-woff2',
    })),
  ];

  return (
    XML_DECLARATION +
    xmlBuilder.build({
      package: {
        '@_xmlns': 'http://www.idpf.org/2007/opf',
        '@_version': '3.0',
        '@_unique-identifier': 'uid',
        metadata: {
          '@_xmlns:dc': 'http://purl.org/dc/elements/1.1/',
          'dc:identifier': { '#text': `urn:uuid:${uuid}`, '@_id': 'uid' },
          'dc:title': title,
          'dc:creator': author,
          'dc:language': 'ko',
          meta: { '#text': modified, '@_property': 'dcterms:modified' },
        },
        manifest: {
          item: manifestItems,
        },
        spine: {
          itemref: { '@_idref': 'document' },
        },
      },
    })
  );
}

export function generateNavXhtml(title: string): string {
  return (
    XML_DECLARATION +
    `<!DOCTYPE html>\n` +
    xmlBuilder.build({
      html: {
        '@_xmlns': 'http://www.w3.org/1999/xhtml',
        '@_xmlns:epub': 'http://www.idpf.org/2007/ops',
        head: { title },
        body: {
          nav: {
            '@_epub:type': 'toc',
            ol: {
              li: {
                a: { '#text': title, '@_href': 'document.xhtml' },
              },
            },
          },
        },
      },
    })
  );
}
