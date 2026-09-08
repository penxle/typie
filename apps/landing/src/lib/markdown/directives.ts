import type { MarkedExtension, Token, TokenizerAndRendererExtension, Tokens } from 'marked';

export type ContainerToken = Tokens.Generic & {
  raw: string;
  label: string;
  tokens: Token[];
};

export type SpaceToken = Tokens.Generic & {
  type: 'blockSpace';
  raw: string;
  lines: number;
};

const spaceFence = /^ {0,3}:{3,}[ \t]*space(?:[ \t]+([1-9]\d*))?[ \t]*(?:\n|$)/;
const anySpaceFence = /^ {0,3}:{3,}[ \t]*space/m;

const chevron =
  '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m9 18 6-6-6-6"/></svg>';

const escapeHtml = (text: string) =>
  text.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll('"', '&quot;');

type ContainerOptions = {
  name: string;
  keyword: string;
  labelled: boolean;
  render: (label: string, body: string) => string;
};

const container = ({ name, keyword, labelled, render }: ContainerOptions): TokenizerAndRendererExtension => {
  const open = new RegExp(String.raw`^ {0,3}(:{3,})[ \t]*${keyword}[ \t]*${labelled ? String.raw`([^\n]*)` : '()'}(?:\n|$)`);
  const anywhere = new RegExp(String.raw`^ {0,3}:{3,}[ \t]*${keyword}`, 'm');

  return {
    name,
    level: 'block',

    start(src: string) {
      return src.match(anywhere)?.index;
    },

    tokenizer(src: string) {
      const match = open.exec(src);
      if (!match) return;

      const rest = src.slice(match[0].length);
      const close = new RegExp(String.raw`^ {0,3}:{${match[1].length},}[ \t]*(?:\n|$)`, 'm').exec(rest);
      const body = close ? rest.slice(0, close.index) : rest;

      return {
        type: name,
        raw: match[0] + (close ? rest.slice(0, close.index + close[0].length) : rest),
        label: match[2].trim(),
        tokens: this.lexer.blockTokens(body),
      };
    },

    renderer(token) {
      return render(escapeHtml(String(token.label ?? '')), this.parser.parse(token.tokens ?? []));
    },
  };
};

export const markedDirectives = (): MarkedExtension => ({
  extensions: [
    container({
      name: 'details',
      keyword: 'details',
      labelled: true,
      render: (label, body) => `<details><summary>${chevron}${label}</summary><div data-details-body>${body}</div></details>`,
    }),

    container({
      name: 'note',
      keyword: 'note',
      labelled: false,
      render: (_, body) => `<div data-note>${body}</div>`,
    }),

    {
      name: 'blockSpace',
      level: 'block',

      start(src: string) {
        return src.match(anySpaceFence)?.index;
      },

      tokenizer(src: string) {
        const match = spaceFence.exec(src);
        if (!match) return;

        return {
          type: 'blockSpace',
          raw: match[0],
          lines: match[1] ? Number(match[1]) : 1,
        } satisfies SpaceToken;
      },

      renderer(token) {
        const { lines } = token as SpaceToken;
        return `<div data-space style="height:${lines * 1.75}em"></div>`;
      },
    },
  ],
});
