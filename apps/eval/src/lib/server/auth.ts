type AuthInput = {
  pathname: string;
  authorizationHeader: string | null;
  accessEmailHeader: string | null;
  ingestToken: string;
  devEmail?: string;
  adminEmails?: string;
};

type AuthResult = { kind: 'runner' } | { kind: 'evaluator'; email: string } | { kind: 'denied'; status: 401 | 403 };

const runnerPaths = ['/api/ingest/', '/api/rounds'];
const adminPathPrefixes = ['/admin', '/dashboard'];

export const resolveAuth = (input: AuthInput): AuthResult => {
  if (runnerPaths.some((p) => input.pathname.startsWith(p))) {
    if (input.authorizationHeader === `Bearer ${input.ingestToken}`) {
      return { kind: 'runner' };
    }
    return { kind: 'denied', status: 401 };
  }

  const email = input.accessEmailHeader ?? input.devEmail;
  if (!email) {
    return { kind: 'denied', status: 403 };
  }

  if (adminPathPrefixes.some((p) => input.pathname.startsWith(p))) {
    const allowed = (input.adminEmails ?? '')
      .split(',')
      .map((e) => e.trim())
      .filter((e) => e.length > 0);
    if (!allowed.includes(email)) {
      return { kind: 'denied', status: 403 };
    }
  }

  return { kind: 'evaluator', email };
};
