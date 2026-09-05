import { describe, expect, it } from 'vitest';
import { resolveThemeAttributes } from './theme-ssr';

describe('resolveThemeAttributes', () => {
  it('follows cookies on ordinary routes', () => {
    expect(
      resolveThemeAttributes({
        routeId: '/website/(dashboard)/[slug]',
        pathname: '/abc',
        cookies: { theme: 'dark', light: 'flexoki', dark: 'nord' },
      }),
    ).toEqual({ theme: 'dark', variantLight: 'flexoki', variantDark: 'nord' });
  });

  it('defaults to auto/white/black without cookies', () => {
    expect(resolveThemeAttributes({ routeId: '/website/(dashboard)', pathname: '/', cookies: {} })).toEqual({
      theme: 'auto',
      variantLight: 'white',
      variantDark: 'black',
    });
  });

  it('ignores invalid theme cookie values', () => {
    expect(resolveThemeAttributes({ routeId: '/website/(dashboard)', pathname: '/', cookies: { theme: 'sepia' } }).theme).toBe('auto');
  });

  it('forces light on webview routes', () => {
    expect(resolveThemeAttributes({ routeId: '/website/_webview/x', pathname: '/_webview/x', cookies: {} }).theme).toBe('light');
  });

  it('forces dark black on landing routes regardless of cookies', () => {
    for (const routeId of ['/website/(landing)/(index)', '/website/(landing)/pricing']) {
      expect(resolveThemeAttributes({ routeId, pathname: '/', cookies: { theme: 'light', light: 'flexoki', dark: 'nord' } })).toEqual({
        theme: 'dark',
        variantLight: 'white',
        variantDark: 'black',
      });
    }
  });

  it('forces dark black on admin routes regardless of cookies', () => {
    expect(
      resolveThemeAttributes({
        routeId: '/website/admin/users',
        pathname: '/',
        cookies: { theme: 'light', light: 'flexoki', dark: 'nord' },
      }),
    ).toEqual({
      theme: 'dark',
      variantLight: 'white',
      variantDark: 'black',
    });
  });

  it('falls back to the default variants for unknown cookie values', () => {
    expect(resolveThemeAttributes({ routeId: '/website/(dashboard)', pathname: '/', cookies: { light: 'sepia', dark: 'sepia' } })).toEqual({
      theme: 'auto',
      variantLight: 'white',
      variantDark: 'black',
    });
  });

  it('forces dark black on legal routes regardless of cookies', () => {
    for (const routeId of ['/website/legal/terms', '/website/legal/privacy']) {
      expect(resolveThemeAttributes({ routeId, pathname: '/', cookies: { theme: 'light', light: 'flexoki', dark: 'nord' } })).toEqual({
        theme: 'dark',
        variantLight: 'white',
        variantDark: 'black',
      });
    }
  });

  it('does not force dark on usersite or auth routes', () => {
    expect(resolveThemeAttributes({ routeId: '/usersite/wildcard/[slug]', pathname: '/', cookies: { theme: 'light' } }).theme).toBe(
      'light',
    );
    expect(resolveThemeAttributes({ routeId: '/auth/login', pathname: '/login', cookies: { theme: 'light' } }).theme).toBe('light');
  });
});
