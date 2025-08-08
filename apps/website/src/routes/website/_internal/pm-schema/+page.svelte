<script lang="ts">
  import { getSchema } from '@tiptap/core';
  import { baseExtensions } from '@typie/ui/tiptap';

  const schema = getSchema(baseExtensions);
  const spec = {
    nodes: schema.spec.nodes.toObject(),
    marks: schema.spec.marks.toObject(),
    topNode: schema.spec.topNode,
  };
</script>

{JSON.stringify(spec)}
