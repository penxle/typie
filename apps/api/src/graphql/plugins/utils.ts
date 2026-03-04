export const truncateVariables = (variables: Record<string, unknown> | null | undefined): unknown => {
  return truncateValue(variables);
};

const truncateValue = (value: unknown): unknown => {
  if (value === null || value === undefined) return value;

  if (typeof value === 'string') {
    return value.length > 200 ? value.slice(0, 200) + `... (${value.length} bytes)` : value;
  }

  if (typeof value !== 'object') return value;

  if (Array.isArray(value)) {
    const items = value.slice(0, 5).map(truncateValue);
    if (value.length > 5) {
      items.push(`... and ${value.length - 5} more`);
    }
    return items;
  }

  const result: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(value)) {
    result[k] = truncateValue(v);
  }
  return result;
};
