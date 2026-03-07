import { redirect } from '@sveltejs/kit';
import { serializeOAuthState } from '@typie/ui/utils';
import qs from 'query-string';
import { env } from '$env/dynamic/public';
import { checkBootstrapAssertion } from '$lib/bootstrap';
import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';
import type { LayoutLoad } from './$types';

export const ssr = false;

export const load: LayoutLoad = async (event) => {
  await checkBootstrapAssertion(event.fetch);

  const query = await loadQuery(
    event,
    graphql(`
      query DashboardLayout_Query {
        me @required {
          id
          name
          email
          preferences

          avatar {
            id
            url
          }

          sites {
            id
            name

            ...DashboardLayout_SiteSettingsModal_site
            ...DashboardLayout_TrashModal_site
          }

          referral {
            id
          }

          surveys
          marketingConsentAskedAt

          usage {
            totalCharacterCount
            totalBlobSize
          }

          subscription {
            id

            plan {
              id

              rule {
                maxTotalCharacterCount
                maxTotalBlobSize
              }
            }
          }

          documentFontFamilies {
            id
            familyName
            displayName
            source
            state

            fonts {
              id
              weight
              state
              subfamilyDisplayName
            }
          }

          textReplacements {
            __typename
            ... on TextReplacement {
              id
              match
              substitute
              regex
            }
            ... on TextReplacementPreference {
              id
              state
              textReplacement {
                id
                match
                substitute
                regex
              }
            }
          }

          ...DashboardLayout_CommandPalette_user
          ...DashboardLayout_PreferenceModal_user
          ...DashboardLayout_Sidebar_user
          ...DashboardLayout_SiteSettingsModal_user
          ...DashboardLayout_TrialExpiredModal_user
        }

        ...AdminImpersonateBanner_query
        ...DashboardLayout_Shortcuts_query
      }
    `),
  );

  if (!query.data.me) {
    redirect(
      302,
      qs.stringifyUrl({
        url: `${env.PUBLIC_AUTH_URL}/authorize`,
        query: {
          client_id: env.PUBLIC_OIDC_CLIENT_ID,
          response_type: 'code',
          redirect_uri: `${env.PUBLIC_WEBSITE_URL}/authorize`,
          state: serializeOAuthState({ redirect_uri: event.url.href }),
        },
      }),
    );
  }

  return { query };
};
