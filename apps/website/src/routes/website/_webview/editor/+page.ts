import type { WebViewEditorPage_Query_Variables } from './$graphql';

export const _WebViewEditorPage_Query_Variables: WebViewEditorPage_Query_Variables = ({ url }) => ({
  // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
  slug: url.searchParams.get('slug')!,
});
