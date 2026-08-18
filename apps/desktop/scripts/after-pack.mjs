import { mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';

const DISPLAY_NAME = '타이피';
const LOCALES = ['ko', 'en'];

// eslint-disable-next-line import/no-default-export
export default async function afterPack(context) {
  if (context.electronPlatformName !== 'darwin') {
    return;
  }

  const appPath = path.join(context.appOutDir, `${context.packager.appInfo.productFilename}.app`);
  const resources = path.join(appPath, 'Contents', 'Resources');
  const content = `CFBundleName = "${DISPLAY_NAME}";\nCFBundleDisplayName = "${DISPLAY_NAME}";\n`;

  for (const locale of LOCALES) {
    const dir = path.join(resources, `${locale}.lproj`);
    await mkdir(dir, { recursive: true });
    await writeFile(path.join(dir, 'InfoPlist.strings'), content, 'utf8');
  }
}
