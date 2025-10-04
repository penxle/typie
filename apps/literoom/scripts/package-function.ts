import { $ } from 'execa';

const $func = $({ cwd: 'dist/function' });
await $func`zip -r ../function.zip .`;

console.log('Function packaged');
