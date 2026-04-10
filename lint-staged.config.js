export default {
  '*.{ts,tsx,js,jsx,svelte}': ['eslint --fix --quiet', 'prettier --write --ignore-unknown'],
  '*.{json,yaml,yml,md,graphql}': ['prettier --write --ignore-unknown --no-error-on-unmatched-pattern'],
  '*': ['cspell --no-progress --relative --no-must-find-files'],
  '*.dart': (files) => [...files.map((f) => `dart fix --apply "${f}"`), `dart format ${files.join(' ')}`],
  '*.rs': ['cargo fmt --'],
  '*.{kt,kts}': ['ktfmt'],
};
