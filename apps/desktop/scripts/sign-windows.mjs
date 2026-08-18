import { execFile } from 'node:child_process';
import { promisify } from 'node:util';

const exec = promisify(execFile);

const TOKEN_SCOPE = 'https://codesigning.azure.net/.default';
const TIMESTAMP_URL = 'http://timestamp.acs.microsoft.com';

let cachedToken = null;

const readEnv = () => {
  const { AZURE_TENANT_ID, AZURE_CLIENT_ID, AZURE_CLIENT_SECRET, AZURE_SIGNING_ENDPOINT, AZURE_SIGNING_ACCOUNT, AZURE_SIGNING_PROFILE } =
    process.env;

  if (!AZURE_TENANT_ID || !AZURE_CLIENT_ID || !AZURE_CLIENT_SECRET) {
    return null;
  }

  for (const [name, value] of Object.entries({ AZURE_SIGNING_ENDPOINT, AZURE_SIGNING_ACCOUNT, AZURE_SIGNING_PROFILE })) {
    if (!value) {
      throw new Error(`${name} is required for Windows signing`);
    }
  }

  return {
    tenantId: AZURE_TENANT_ID,
    clientId: AZURE_CLIENT_ID,
    clientSecret: AZURE_CLIENT_SECRET,
    endpoint: AZURE_SIGNING_ENDPOINT.replace(/\/$/, ''),
    account: AZURE_SIGNING_ACCOUNT,
    profile: AZURE_SIGNING_PROFILE,
  };
};

const getToken = async ({ tenantId, clientId, clientSecret }) => {
  if (cachedToken && cachedToken.expiresAt > Date.now() + 60_000) {
    return cachedToken.value;
  }

  const response = await fetch(`https://login.microsoftonline.com/${tenantId}/oauth2/v2.0/token`, {
    method: 'POST',
    headers: { 'content-type': 'application/x-www-form-urlencoded' },
    body: new URLSearchParams({ client_id: clientId, client_secret: clientSecret, scope: TOKEN_SCOPE, grant_type: 'client_credentials' }),
  });

  if (!response.ok) {
    throw new Error(`Azure token request failed: ${response.status} ${await response.text()}`);
  }

  const { access_token: value, expires_in: expiresIn } = await response.json();
  cachedToken = { value, expiresAt: Date.now() + expiresIn * 1000 };

  return value;
};

// eslint-disable-next-line import/no-default-export
export default async function sign(configuration) {
  const env = readEnv();
  if (!env) {
    console.log(`  • skipping Windows signing (no Azure credentials)  file=${configuration.path}`);
    return;
  }

  if (!configuration.options.signtoolOptions?.publisherName) {
    throw new Error('win.signtoolOptions.publisherName is required for Windows signing');
  }

  const token = await getToken(env);

  await exec(
    'jsign',
    [
      '--storetype',
      'TRUSTEDSIGNING',
      '--keystore',
      env.endpoint,
      '--storepass',
      'env:AZURE_SIGNING_TOKEN',
      '--alias',
      `${env.account}/${env.profile}`,
      '--tsaurl',
      TIMESTAMP_URL,
      '--tsmode',
      'RFC3161',
      configuration.path,
    ],
    { env: { ...process.env, AZURE_SIGNING_TOKEN: token } },
  );
}
