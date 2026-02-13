import 'dart:async';

import 'package:typie/native/slate_reader.dart';
import 'package:typie/screens/native_editor/external/models.dart';
import 'package:typie/screens/native_editor/state/controller.dart';
import 'package:typie/screens/native_editor/state/scroll_mode.dart';
import 'package:typie/screens/native_editor/state/state.dart';

class CommandHandler {
  static void handleSlate(EditorController controller, SlateReader reader) {
    final dirty = reader.dirty;
    if (dirty == 0) {
      return;
    }

    if (dirty & (1 << 15) != 0) {
      _handleDocChanged(controller);
    }

    if (dirty & (1 << 16) != 0) {
      _handleRenderRequired(controller);
    }

    if (dirty & (1 << 0) != 0) {
      _handleSettingsChanged(controller, reader);
    }

    if (dirty & (1 << 1) != 0) {
      _handleLayoutChanged(controller, reader);
    }

    if (dirty & (1 << 2) != 0) {
      _handleCursorChanged(controller, reader);
    }

    if (dirty & (1 << 3) != 0) {
      _handleSelectionChanged(controller, reader);
    }

    if (dirty & (1 << 4) != 0) {
      _handleAttrsChanged(controller, reader);
    }

    if (dirty & (1 << 7) != 0) {
      _handlePlaceholderChanged(controller, reader);
    }

    if (dirty & (1 << 8) != 0) {
      _handleExternalElements(controller, reader);
    }

    if (dirty & (1 << 11) != 0) {
      _handleTrackedItemsChanged(controller, reader);
    }

    if (dirty & (1 << 17) != 0) {
      _handleFontRequired(controller, reader);
    }

    if (dirty & (1 << 18) != 0) {
      _handleFallbackFontRequired(controller, reader);
    }

    if (dirty & (1 << 19) != 0) {
      _handleExitedDocumentStart(controller);
    }

    if (dirty & (1 << 20) != 0) {
      _handleHtmlPasted(controller, reader);
    }
  }

  static void _handleDocChanged(EditorController controller) {
    controller.onDocChanged?.call();
    controller.pendingScrollMode = ScrollMode.typewriter;
  }

  static void _handleRenderRequired(EditorController controller) {
    controller.updateState((state) => state.copyWith(renderVersion: Object()));
  }

  static void _handleSettingsChanged(EditorController controller, SlateReader reader) {
    final paragraphIndent = reader.getF32('paragraph_indent');
    final blockGap = reader.getF32('block_gap');

    controller.updateState(
      (state) => state.copyWith(
        settings: state.settings.copyWith(paragraphIndent: paragraphIndent, blockGap: blockGap),
      ),
    );
  }

  static void _handleLayoutChanged(EditorController controller, SlateReader reader) {
    final pagesCount = reader.getU32('pages_count');
    final pagesOffset = reader.getU32('pages_offset');
    final raw = reader.readF32List(pagesOffset, pagesCount * 2);
    final pages = [for (var i = 0; i < pagesCount; i++) PageSize(width: raw[i * 2], height: raw[i * 2 + 1])];

    var lmPos = reader.getU32('layout_mode_offset');
    final layoutModeTag = reader.slabU32(lmPos);
    lmPos += 4;

    final LayoutModeInfo layoutMode;

    if (layoutModeTag == 0) {
      layoutMode = LayoutModeInfo.paginated(
        pageWidth: reader.slabF32(lmPos),
        pageHeight: reader.slabF32(lmPos + 4),
        pageMarginTop: reader.slabF32(lmPos + 8),
        pageMarginBottom: reader.slabF32(lmPos + 12),
        pageMarginLeft: reader.slabF32(lmPos + 16),
        pageMarginRight: reader.slabF32(lmPos + 20),
      );
    } else {
      layoutMode = LayoutModeInfo.continuous(maxWidth: reader.slabF32(lmPos));
    }

    final hadLayout = controller.state.layout != null;

    controller.updateState(
      (state) => state.copyWith(
        layout: LayoutInfo(isPaginated: layoutModeTag == 0, pages: pages, layoutMode: layoutMode),
        renderVersion: Object(),
      ),
    );

    if (!hadLayout && pages.isNotEmpty) {
      controller.onEditorReady?.call();
    }
  }

  static void _handleCursorChanged(EditorController controller, SlateReader reader) {
    final pageIdx = reader.getI32('cursor_page_idx');
    final precedingOffset = reader.getU32('preceding_char_widths_offset');
    final precedingCount = reader.getU32('preceding_char_widths_count');

    final cursor = CursorInfo(
      pageIdx: pageIdx,
      x: reader.getF32('cursor_x'),
      y: reader.getF32('cursor_y'),
      height: reader.getF32('cursor_height'),
      visible: reader.getU32('cursor_visible') != 0,
      precedingCharWidths: reader.readF32List(precedingOffset, precedingCount),
    );

    controller.updateState((state) => state.copyWith(cursor: cursor));
  }

  static void _handleSelectionChanged(EditorController controller, SlateReader reader) {
    final cmp = reader.getI32('selection_cmp');
    final collapsed = cmp == 0;

    SelectionHandleInfo? fromHandleInfo;
    SelectionHandleInfo? toHandleInfo;

    if (!collapsed) {
      final anchorPageIdx = reader.getI32('selection_anchor_page_idx');
      final anchorHandle = anchorPageIdx < 0
          ? null
          : SelectionHandleInfo(
              pageIdx: anchorPageIdx,
              x: reader.getF32('selection_anchor_x'),
              y: reader.getF32('selection_anchor_y'),
              height: reader.getF32('selection_anchor_height'),
            );

      final headPageIdx = reader.getI32('selection_head_page_idx');
      final headHandle = headPageIdx < 0
          ? null
          : SelectionHandleInfo(
              pageIdx: headPageIdx,
              x: reader.getF32('selection_head_x'),
              y: reader.getF32('selection_head_y'),
              height: reader.getF32('selection_head_height'),
            );

      if (cmp < 0) {
        fromHandleInfo = headHandle;
        toHandleInfo = anchorHandle;
      } else {
        fromHandleInfo = anchorHandle;
        toHandleInfo = headHandle;
      }
    }

    controller.updateState(
      (state) => state.copyWith(fromHandle: fromHandleInfo, toHandle: toHandleInfo, pasteOptions: null),
    );

    final anchorNodeId = reader.readNodeIdField('selection_anchor_node_id');
    final headNodeId = reader.readNodeIdField('selection_head_node_id');
    final anchorOffset = reader.getU32('selection_anchor_offset');
    final headOffset = reader.getU32('selection_head_offset');
    final anchorAffinity = reader.getU32('selection_anchor_affinity');
    final headAffinity = reader.getU32('selection_head_affinity');

    final anchor = <String, dynamic>{
      'nodeId': anchorNodeId,
      'offset': anchorOffset,
      'affinity': anchorAffinity == 1 ? 'downstream' : 'upstream',
    };
    final head = <String, dynamic>{
      'nodeId': headNodeId,
      'offset': headOffset,
      'affinity': headAffinity == 1 ? 'downstream' : 'upstream',
    };

    controller.onSelectionChanged?.call(anchor, head);
  }

  static void _handleAttrsChanged(EditorController controller, SlateReader reader) {
    final attrs = reader.readAttrs();
    controller.updateState((state) => state.copyWith(attrs: attrs));
  }

  static void _handlePlaceholderChanged(EditorController controller, SlateReader reader) {
    final visible = reader.getU32('placeholder_visible') != 0;

    controller.updateState(
      (state) => state.copyWith(
        placeholder: PlaceholderInfo(
          visible: visible,
          x: visible ? reader.getF32('placeholder_x') : null,
          y: visible ? reader.getF32('placeholder_y') : null,
          width: visible ? reader.getF32('placeholder_width') : null,
          height: visible ? reader.getF32('placeholder_height') : null,
        ),
      ),
    );
  }

  static void _handleExternalElements(EditorController controller, SlateReader reader) {
    final rawElements = reader.readExternalElements();
    final elements = rawElements.map((e) {
      final ExternalElementData data;
      switch (e.dataTag) {
        case 0:
          data = ExternalElementData.image(id: e.id, proportion: e.proportion, uploadId: e.uploadId);
        case 1:
          data = ExternalElementData.file(id: e.id, uploadId: e.uploadId);
        case 2:
          data = ExternalElementData.embed(id: e.id);
        default:
          data = ExternalElementData.archived(id: e.id);
      }
      return ExternalElement(
        pageIdx: e.pageIdx,
        nodeId: e.nodeId,
        bounds: ExternalElementBounds(x: e.x, y: e.y, width: e.width, height: e.height),
        data: data,
        isSelected: e.isSelected,
      );
    }).toList();

    controller.updateState((state) => state.copyWith(externalElements: elements));
  }

  static void _handleTrackedItemsChanged(EditorController controller, SlateReader reader) {
    final rawItems = reader.readTrackedItems();

    final spellcheckOverlays = <SpellcheckOverlayInfo>[];
    final aiFeedbackOverlays = <AiFeedbackOverlayInfo>[];
    final searchOverlays = <SearchOverlayInfo>[];

    for (final item in rawItems) {
      if (item.group == 0) {
        final bounds = item.bounds
            .map((b) => SpellcheckOverlayBound(x: b.x, y: b.y, width: b.width, height: b.height, ascent: b.ascent))
            .toList();
        spellcheckOverlays.add(
          SpellcheckOverlayInfo(pageIdx: item.pageIdx, id: item.id, isActive: false, bounds: bounds),
        );
      } else if (item.group == 1) {
        final bounds = item.bounds
            .map((b) => AiFeedbackOverlayBound(x: b.x, y: b.y, width: b.width, height: b.height))
            .toList();
        aiFeedbackOverlays.add(
          AiFeedbackOverlayInfo(pageIdx: item.pageIdx, id: item.id, isActive: false, bounds: bounds),
        );
      } else if (item.group == 2) {
        final rects = item.bounds
            .map((b) => SearchHighlightRect(x: b.x, y: b.y, width: b.width, height: b.height))
            .toList();
        searchOverlays.add(SearchOverlayInfo(pageIdx: item.pageIdx, isCurrent: false, bounds: rects));
      }
    }

    controller.updateState(
      (state) => state.copyWith(
        spellcheck: SpellcheckState(overlays: spellcheckOverlays),
        aiFeedback: AiFeedbackState(overlays: aiFeedbackOverlays),
        search: state.search.copyWith(overlays: searchOverlays),
      ),
    );
  }

  static void _handleFontRequired(EditorController controller, SlateReader reader) {
    final manager = controller.fontManager;
    if (manager == null) {
      return;
    }

    final requests = reader.readFontRequests();
    for (final req in requests) {
      unawaited(
        manager.ensureRequiredFont(req.family, req.weight, req.codepoints).then((_) {
          controller.dispatch({'type': 'fontsLoaded'});
        }),
      );
    }
  }

  static void _handleFallbackFontRequired(EditorController controller, SlateReader reader) {
    final manager = controller.fontManager;
    if (manager == null) {
      return;
    }

    final codepoints = reader.readFallbackCodepoints();
    if (codepoints.isEmpty) {
      return;
    }

    unawaited(
      manager.ensureRequiredFallbackFont(codepoints).then((_) {
        controller.dispatch({'type': 'fontsLoaded'});
      }),
    );
  }

  static void _handleExitedDocumentStart(EditorController controller) {
    controller.onExitedDocumentStart?.call();
  }

  static void _handleHtmlPasted(EditorController controller, SlateReader reader) {
    final pasted = reader.readHtmlPasted();
    if (pasted == null) {
      return;
    }

    controller.updateState(
      (state) => state.copyWith(
        pasteOptions: PasteOptionsInfo(
          text: pasted.text,
          from: {'nodeId': pasted.fromNodeId, 'offset': pasted.fromOffset, 'affinity': pasted.fromAffinity},
          to: {'nodeId': pasted.toNodeId, 'offset': pasted.toOffset, 'affinity': pasted.toAffinity},
        ),
      ),
    );
  }
}
