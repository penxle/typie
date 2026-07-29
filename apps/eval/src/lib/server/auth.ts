type AuthInput = {
  pathname: string;
  accessEmailHeader: string | null;
  devEmail?: string;
  adminEmails?: string;
};

type AuthResult = { kind: 'evaluator'; email: string } | { kind: 'denied'; status: 403 };

const adminPathPrefixes = ['/admin'];

// 들어오는 길은 Access 하나뿐이다. 러너용 Bearer 경로는 워크플로가 D1에 직접 쓰면서 사라졌다.
export const resolveAuth = (input: AuthInput): AuthResult => {
  const email = input.accessEmailHeader ?? input.devEmail;
  if (!email) {
    return { kind: 'denied', status: 403 };
  }

  if (adminPathPrefixes.some((p) => input.pathname.startsWith(p)) && !isAdmin({ ADMIN_EMAILS: input.adminEmails }, email)) {
    return { kind: 'denied', status: 403 };
  }

  return { kind: 'evaluator', email };
};

export const isAdmin = (env: { ADMIN_EMAILS?: string }, email: string): boolean =>
  (env.ADMIN_EMAILS ?? '')
    .split(',')
    .map((e) => e.trim())
    .filter((e) => e.length > 0)
    .includes(email);
