# 프로젝트 전체 Linting

다음 내용에 따라 Typie 프로젝트 전체에 lint를 수행하세요.

## 1단계: 전체 Lint 검사

다음 lint 명령들을 모두 병렬로 실행하여 현재 상태를 확인하세요:

- `pnpm lint:eslint`
- `pnpm lint:prettier`
- `pnpm lint:spellcheck`
- `pnpm lint:svelte`
- `pnpm lint:syncpack`
- `pnpm lint:typecheck`

## 2단계: 자동 수정

오류가 발견되면 다음 자동 수정 명령을 실행하세요:

### ESLint 자동 수정

```bash
pnpm eslint . --fix
```

### Prettier 자동 수정

```bash
pnpm prettier --write .
```

### Syncpack 자동 수정

```bash
pnpm syncpack fix-mismatches
```

## 3단계: 수동 수정

자동 수정이 불가능한 오류는 직접 수정하세요.

## 4단계: 최종 검증

모든 수정 후 다시 전체 lint를 실행하여 오류가 없는지 확인하세요.

## Lint 도구 참고사항

| 도구         | 용도                 | 자동 수정 |
| ------------ | -------------------- | --------- |
| ESLint       | JS/TS 문법 및 스타일 | ✅        |
| Prettier     | 코드 포맷팅          | ✅        |
| Spellcheck   | 맞춤법 검사          | ❌        |
| Svelte Check | Svelte 컴포넌트 검증 | ❌        |
| Syncpack     | 의존성 버전 일관성   | ✅        |
| TypeScript   | 타입 검사            | ❌        |

## 주의사항

- 모든 lint 오류가 해결될 때까지 작업을 계속하세요
- 자동 수정 가능한 도구는 반드시 자동 수정을 먼저 시도하세요
