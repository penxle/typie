import DOMPurify from 'isomorphic-dompurify';

export const sanitizeHighlight = (dirty: string | undefined) => {
  return dirty ? DOMPurify.sanitize(dirty, { ALLOWED_TAGS: ['em'], ALLOWED_ATTR: [] }) : undefined;
};
