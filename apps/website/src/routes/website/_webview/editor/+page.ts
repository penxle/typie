/* eslint-disable @typescript-eslint/no-non-null-assertion */

import type { WebViewEditorPage_Query_Variables } from './$graphql';

export const _WebViewEditorPage_Query_Variables: WebViewEditorPage_Query_Variables = ({ url }) => ({
  slug: url.searchParams.get('slug')!,
  siteId: url.searchParams.get('siteId')!,
});
