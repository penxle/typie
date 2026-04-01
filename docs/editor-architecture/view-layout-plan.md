# View Layout System — Implementation Summary

구현 완료된 계획의 최종 상태 기록.

## File Structure

```
editor-common/src/
├── geometry.rs             Rect (bottom/right/center_x/contains), Size
├── movement.rs             Movement, Direction (editor-core에서 이동)
└── lib.rs

editor-view/src/
├── lib.rs
├── view.rs                 View facade
├── viewport.rs             Viewport { width, scale_factor }
├── view_state.rs           ViewState { fold_states, external_heights }
├── page.rs                 Page { fragments, height }
├── fragment/
│   ├── mod.rs              Fragment enum, navigate_to(), position_in_line()
│   ├── container.rs        ContainerFragment, Breaks
│   ├── line.rs             LineFragment, LineSegment
│   └── atom.rs             AtomFragment
├── measure/
│   ├── mod.rs              Measurement, MeasuredContent, LayoutDirection, ChildMeasurement
│   └── line.rs             MeasuredLine
├── engine/
│   ├── mod.rs              LayoutEngine (cache, invalidate, compute, measure)
│   ├── cache.rs            LayoutCache (FxHashMap<NodeId, Arc<Measurement>>)
│   └── paginator.rs        Paginator (Mode, Container stack, page/tile splitting)
└── cursor/
    ├── mod.rs              cursor_rect, x_at_offset
    ├── hit_test.rs         hit_test
    ├── navigation.rs       resolve_movement (Grapheme, Line)
    └── search.rs           find_navigable_at_y, find_navigable_below/above, find_line_at

editor-core/src/
├── message.rs              pub use editor_common::{Direction, Movement}
└── editor.rs               Editor uses View
```

## Stub으로 남은 것

| 항목 | 상태 |
|---|---|
| `measure_inner()` | 모든 노드를 빈 Container로 측정. 레거시 레이아웃 코드 포팅 필요. |
| `Movement::Word, Sentence, Block, Page, Document` | None 반환. |
| 다단 레이아웃 | 단일 컬럼만. |
| 렌더링 | 범위 밖. |

## Test Coverage

95 tests (65 editor-view + 30 editor-core).

Paginator: 32 tests covering paginated/continuous modes, margins, gap collapsing/preservation, Breaks, wrapper extension, horizontal containers, page breaks, empty documents, tile splitting, hierarchy preservation, empty container skipping, leaf container atomic placement.
