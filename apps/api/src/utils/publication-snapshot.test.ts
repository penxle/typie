import assert from 'node:assert/strict';
import test from 'node:test';
import { buildPublicationSnapshot } from './publication-snapshot.ts';
import { wasm } from './wasm-ffi.ts';

const PARAGRAPHS = ['발행 스냅숏 테스트 문단입니다.', '두 번째 문단을 덧붙인다.'];

const concat = (a: Uint8Array, b: Uint8Array) => {
  const merged = new Uint8Array(a.length + b.length);
  merged.set(a, 0);
  merged.set(b, a.length);
  return merged;
};

const historyBearingGraph = async () =>
  await wasm.use((host) => {
    let graph = host.to_graph(host.default_doc_with_preset({ layout_mode: { type: 'continuous', max_width: 600 } }, []));

    for (const text of PARAGRAPHS) {
      const { xml } = host.to_xml(graph, []);
      const result = host.edit_from_xml(
        graph,
        [],
        xml.replace('</root>', () => `  <paragraph>${text}</paragraph>\n</root>`),
      );

      assert.equal(result.error, undefined);
      assert.ok(result.chars_inserted > 0);

      graph = concat(graph, result.bundle);
    }

    return graph;
  });

test('얕은 판은 히스토리가 있는 문서를 같은 문서로 재구성한다', async () => {
  const graph = await historyBearingGraph();
  const snapshot = await buildPublicationSnapshot(graph);

  assert.ok(snapshot.graph.length > 0);
  assert.ok(snapshot.graph.length < graph.length);
  assert.equal(snapshot.plain.root.node.type, 'root');

  await wasm.use((host) => {
    const original = host.materialize(graph);

    assert.ok(snapshot.characterCount > 0);
    assert.equal(snapshot.characterCount, host.count_characters(original.text));
    assert.equal(snapshot.text, original.text);
    assert.deepEqual(snapshot.heads, host.heads(graph));

    const roundTrip = host.materialize(snapshot.graph);

    assert.equal(roundTrip.projection_degraded, false);
    assert.equal(roundTrip.text, snapshot.text);
    assert.deepEqual(roundTrip.plain, original.plain);
  });
});

test('얕은 판 자체도 heads를 갖는 유효한 그래프다', async () => {
  const graph = await historyBearingGraph();
  const snapshot = await buildPublicationSnapshot(graph);
  const shallowHeads = await wasm.use((host) => host.heads(snapshot.graph));

  assert.ok(shallowHeads.length > 0);
});
