<script lang="ts">
  import { getSchema } from '@tiptap/core';
  import { extensions } from '$lib/tiptap';

  const schema = getSchema(extensions);
  const spec = {
    nodes: schema.spec.nodes.toObject(),
    marks: schema.spec.marks.toObject(),
    topNode: schema.spec.topNode,
  };
</script>

{JSON.stringify(spec)}
