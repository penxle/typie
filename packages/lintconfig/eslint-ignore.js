import fs from 'node:fs';
import path from 'node:path';
import ig from 'ignore';

const findGitignore = () => {
  let currentDir = process.cwd();

  while (true) {
    const gitignorePath = path.join(currentDir, '.gitignore');

    if (fs.existsSync(gitignorePath)) {
      return { path: gitignorePath, dir: currentDir };
    }

    const parentDir = path.dirname(currentDir);
    if (parentDir === currentDir) {
      throw new Error('.gitignore file not found');
    }

    currentDir = parentDir;
  }
};

const gitignoreInfo = findGitignore();
const matcher = ig().add(fs.readFileSync(gitignoreInfo.path, 'utf8'));

export const ignore = (p) => {
  const relative = path.relative(gitignoreInfo.dir, p);

  if (relative.startsWith('..')) {
    return false;
  }

  if (relative.length === 0) {
    return false;
  }

  return matcher.ignores(relative);
};
