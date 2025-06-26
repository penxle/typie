import postcss from 'postcss';

/**
 * @returns {import('postcss').Plugin}
 */
const plugin = () => {
  return {
    postcssPlugin: '@typie/lib/postcss',
    Once(root) {
      const darkThemeRules = [];

      root.walkRules((rule) => {
        if (/\[data-theme=['"]?dark['"]?\]/.test(rule.selector)) {
          darkThemeRules.push(rule);
        }
      });

      darkThemeRules.forEach((rule) => {
        let mediaSelector;

        if (/^\[data-theme=['"]?dark['"]?\]$/.test(rule.selector)) {
          mediaSelector = ':root:not([data-theme="light"])';
        } else {
          const baseSelector = rule.selector.replace(/\[data-theme=["']?dark["']?\]\s*/, '');
          mediaSelector = `${baseSelector}:not([data-theme="light"] *)`;
        }

        const mediaRule = postcss.atRule({ name: 'media', params: '(prefers-color-scheme: dark)' });
        const innerRule = postcss.rule({ selector: mediaSelector });

        rule.each((node) => {
          if (node.type === 'decl') {
            innerRule.append(node.clone());
          }
        });

        mediaRule.append(innerRule);
        rule.parent.insertAfter(rule, mediaRule);
      });
    },
  };
};

plugin.postcss = true;

// eslint-disable-next-line import/no-default-export
export default plugin;
