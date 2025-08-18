import { eq } from 'drizzle-orm';
import { chromium } from 'playwright';
import { db, Entities, firstOrThrowWith, Posts, TableCode, validateDbId } from '@/db';
import { EntityVisibility } from '@/enums';
import { env } from '@/env';
import { NotFoundError } from '@/errors';
import { mergePDFs } from '@/utils/pdf';
import { builder } from '../builder';

/**
 * * Types
 */

const ExportPostAsPdfResult = builder.objectRef<{
  data: Buffer;
  filename: string;
}>('ExportPostAsPdfResult');

ExportPostAsPdfResult.implement({
  fields: (t) => ({
    data: t.field({
      type: 'String',
      resolve: (self) => self.data.toString('base64'),
    }),
    filename: t.exposeString('filename'),
  }),
});

/**
 * * Mutations
 */

builder.mutationFields((t) => ({
  exportPostAsPdf: t.withAuth({ session: true }).fieldWithInput({
    type: ExportPostAsPdfResult,
    input: {
      entityId: t.input.id({ validate: validateDbId(TableCode.ENTITIES) }),
      width: t.input.float({ required: false }),
      height: t.input.float({ required: false }),
      marginTop: t.input.float({ required: false }),
      marginBottom: t.input.float({ required: false }),
      marginLeft: t.input.float({ required: false }),
      marginRight: t.input.float({ required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      const entity = await db.select().from(Entities).where(eq(Entities.id, input.entityId)).then(firstOrThrowWith(new NotFoundError()));

      if (entity.visibility === EntityVisibility.PRIVATE && entity.userId !== ctx.session.userId) {
        throw new NotFoundError();
      }

      if (entity.type !== 'POST') {
        throw new NotFoundError();
      }

      const accessToken = ctx.c.req.header('Authorization')?.replace('Bearer ', '');

      const [post, pdfBuffer] = await Promise.all([
        db.select().from(Posts).where(eq(Posts.entityId, entity.id)).then(firstOrThrowWith(new NotFoundError())),
        generatePostPDF({
          entitySlug: entity.slug,
          accessToken,
          pageLayout:
            input.width && input.height && input.marginTop && input.marginBottom && input.marginLeft && input.marginRight
              ? {
                  width: input.width,
                  height: input.height,
                  marginTop: input.marginTop,
                  marginBottom: input.marginBottom,
                  marginLeft: input.marginLeft,
                  marginRight: input.marginRight,
                }
              : undefined,
        }),
      ]);

      return {
        data: pdfBuffer,
        filename: `${post.title || '(내용 없음)'}${post.subtitle ? ` - ${post.subtitle}` : ''}.pdf`,
      };
    },
  }),
}));

/**
 * * Utilities
 */

async function generatePostPDF(params: {
  entitySlug: string;
  accessToken?: string;
  pageLayout?: {
    width: number;
    height: number;
    marginTop: number;
    marginBottom: number;
    marginLeft: number;
    marginRight: number;
  };
}): Promise<Buffer> {
  const { entitySlug, accessToken, pageLayout } = params;

  const browser = await chromium.launch({
    headless: true,
    args: ['--no-sandbox', '--disable-setuid-sandbox'],
  });

  try {
    const urlParams = pageLayout
      ? new URLSearchParams({
          width: pageLayout.width.toString(),
          height: pageLayout.height.toString(),
          'margin-top': pageLayout.marginTop.toString(),
          'margin-bottom': pageLayout.marginBottom.toString(),
          'margin-left': pageLayout.marginLeft.toString(),
          'margin-right': pageLayout.marginRight.toString(),
        })
      : new URLSearchParams();

    const exportUrl = new URL(`/_internal/export/pdf/${entitySlug}`, env.WEBSITE_URL);
    exportUrl.search = urlParams.toString();

    const websiteUrl = new URL(env.WEBSITE_URL);

    const context = await browser.newContext();

    if (accessToken) {
      await context.addCookies([
        {
          name: 'typie-at',
          value: accessToken,
          domain: websiteUrl.hostname,
          path: '/',
          httpOnly: true,
          secure: websiteUrl.protocol === 'https:',
          sameSite: 'Lax',
        },
      ]);
    }

    const page = await context.newPage();

    let resolveIdle: () => void;
    const idlePromise = new Promise<void>((resolve) => {
      resolveIdle = () => resolve();
    });

    await page.exposeFunction('notifyIdle', () => {
      resolveIdle();
    });

    await page.goto(exportUrl.toString(), {
      waitUntil: 'load',
    });

    await page.evaluateHandle('document.fonts.ready');
    await idlePromise;

    if (!pageLayout) {
      // NOTE: scroll 방식인 경우 간단히 생성하고 반환
      const pdfBuffer = await page.pdf({
        printBackground: true,
        displayHeaderFooter: false,
        preferCSSPageSize: false,
        scale: 1,
        margin: {
          top: '20mm',
          bottom: '20mm',
          left: '20mm',
          right: '20mm',
        },
      });

      return Buffer.from(pdfBuffer);
    }

    // NOTE: page layout 방식인 경우 1페이지만큼씩 스크롤하며 PDF 생성하고 병합
    // 웹에 렌더링된 크기와 출력한 PDF 크기가 다르기 때문에 오차가 누적되어 밀리는 현상을 우회하기 위한 동작
    // known issue: https://github.com/puppeteer/puppeteer/issues/4015
    const totalHeight = await page.evaluate(() => {
      const exportPage = document.querySelector('.page-export-viewport');
      return exportPage ? exportPage.scrollHeight : document.body.scrollHeight;
    });

    const pageHeightPx = pageLayout.height * 3.779_527_559_1; // NOTE: Convert mm to px (96 DPI)
    const totalPages = Math.round(totalHeight / pageHeightPx);

    const pdfBuffers: Buffer[] = [];

    for (let i = 0; i < totalPages; i++) {
      await page.evaluate((scrollY) => {
        const exportPage = document.querySelector('.page-export-viewport');
        if (exportPage) {
          exportPage.scrollTop = scrollY;
        } else {
          window.scrollTo(0, scrollY);
        }
      }, i * pageHeightPx);

      const pdfBuffer = await page.pdf({
        printBackground: true,
        displayHeaderFooter: false,
        preferCSSPageSize: false,
        scale: 1,
        width: `${pageLayout.width}mm`,
        height: `${pageLayout.height}mm`,
      });

      pdfBuffers.push(Buffer.from(pdfBuffer));
    }

    return mergePDFs(pdfBuffers);
  } finally {
    await browser.close();
  }
}
