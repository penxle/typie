import type { ExportPdfSlugPage_query_Variables } from './$graphql';

declare global {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Window {
    notifyFontsReady?: () => void;
  }
}

export const _ExportPdfSlugPage_query_Variables: ExportPdfSlugPage_query_Variables = ({ params }) => ({
  slug: params.slug,
});
