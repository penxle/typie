type AuthInput = {
  pathname: string;
  accessEmailHeader: string | null;
  devEmail?: string;
  adminEmails?: string;
};

type AuthResult = { kind: 'evaluator'; email: string } | { kind: 'public' } | { kind: 'denied'; status: 403 };

const adminPathPrefixes = ['/admin'];
// 인증 없이 여는 경로 — 작가에게 링크로 건네는 열람 화면. Cloudflare Access도 이 경로를
// bypass하므로 앱 검사만 남으면 오히려 막힌다. 보호막은 추측 불가능한 runId뿐이다.
const publicPathPrefixes = ['/reads'];

// 들어오는 길은 Access 하나뿐이다. 러너용 Bearer 경로는 워크플로가 D1에 직접 쓰면서 사라졌다.
export const resolveAuth = (input: AuthInput): AuthResult => {
  if (publicPathPrefixes.some((p) => input.pathname === p || input.pathname.startsWith(`${p}/`))) {
    return { kind: 'public' };
  }

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
