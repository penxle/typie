// 평가 참여는 evaluator_consents.evaluating 하나로 정해진다 — 동의만으로는 열리지 않고
// 어드민이 명단에서 켜야 배정이 시작된다(사후승인).
//
// ADMIN_EMAILS는 접근 권한만 뜻하며 참여 여부에 관여하지 않는다. 어드민이면서 평가자일 수
// 있고, 그 표시는 명단 화면의 배지가 전부다.

export const parseEmailList = (raw = ''): Set<string> =>
  new Set(
    raw
      .split(',')
      .map((e) => e.trim())
      .filter((e) => e.length > 0),
  );

export type Consent = { email: string; evaluating: boolean };

export type RosterEntry = Consent & { admin: boolean };

export const buildRoster = (consents: Consent[], adminEmails = ''): RosterEntry[] => {
  const admins = parseEmailList(adminEmails);
  return consents
    .map((consent) => ({ ...consent, admin: admins.has(consent.email) }))
    .toSorted((a, b) => Number(b.evaluating) - Number(a.evaluating) || a.email.localeCompare(b.email));
};
