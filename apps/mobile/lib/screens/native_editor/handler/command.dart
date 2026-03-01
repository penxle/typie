import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:typie/native/slate_reader.dart';
import 'package:typie/screens/native_editor/external/models.dart';
import 'package:typie/screens/native_editor/state/controller.dart';
import 'package:typie/screens/native_editor/state/state.dart';
import 'package:typie/screens/native_editor/table/models.dart';

class CommandHandler {
  static const _selectedHorizontalRuleContext = 'selected_horizontal_rule';
  static const _inCalloutContext = 'in_callout';
  static const _inFoldContext = 'in_fold';
  static const _inBulletListContext = 'in_bullet_list';
  static const _inOrderedListContext = 'in_ordered_list';
  static const _inBlockquoteContext = 'in_blockquote';
  static const _inTableContext = 'in_table';
  static const _selectedImageContext = 'selected_image';
  static const _selectedFileContext = 'selected_file';
  static const _selectedEmbedContext = 'selected_embed';
  static const _selectedArchivedContext = 'selected_archived';
  static const _selectedTableContext = 'selected_table';

  static void handleSlate(EditorController controller, SlateReader reader) {
    final dirty = reader.dirty;
    if (dirty == 0) {
      return;
    }

    controller.beginBatchUpdate();
    try {
      if (dirty & (1 << 15) != 0) {
        _handleDocChanged(controller);
      }

      if (dirty & (1 << 16) != 0) {
        _handleRenderRequired(controller, reader);
      }

      if (dirty & (1 << 0) != 0) {
        _handleSettingsChanged(controller, reader);
      }

      if (dirty & (1 << 1) != 0) {
        _handlePagesChanged(controller, reader);
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

      if (dirty & (1 << 14) != 0) {
        _handleTableOverlaysChanged(controller, reader);
      }

      if (dirty & (1 << 17) != 0) {
        _handleFontRequired(controller, reader);
      }

      if (dirty & (1 << 19) != 0) {
        _handleExitedDocumentStart(controller);
      }

      if (dirty & (1 << 20) != 0) {
        _handleRepaste(controller, reader);
      }

      if (dirty & (1 << 21) != 0) {
        _handleRemarksChanged(controller, reader);
      }
    } finally {
      controller.endBatchUpdate();
    }
  }

  static void _handleDocChanged(EditorController controller) {
    controller.onDocChanged?.call();
    controller.markCharacterCountsDirty();
  }

  static void _handleRenderRequired(EditorController controller, SlateReader reader) {
    final dropIndicator = _readDropIndicator(reader);
    controller.updateState((state) => state.copyWith(renderVersion: Object(), dropIndicator: dropIndicator));
  }

  static DropIndicatorInfo? _readDropIndicator(SlateReader reader) {
    final pageIdx = reader.getI32('drop_indicator_page_idx');
    if (pageIdx < 0) {
      return null;
    }

    return DropIndicatorInfo(
      pageIdx: pageIdx,
      x: reader.getF32('drop_indicator_x'),
      y: reader.getF32('drop_indicator_y'),
      width: reader.getF32('drop_indicator_width'),
      height: reader.getF32('drop_indicator_height'),
    );
  }

  static void _handleSettingsChanged(EditorController controller, SlateReader reader) {
    final paragraphIndent = reader.getU32('paragraph_indent').toDouble();
    final blockGap = reader.getU32('block_gap').toDouble();

    var lmPos = reader.getU32('layout_mode_offset');
    final layoutModeTag = reader.slabU32(lmPos);
    lmPos += 4;

    final Layout layout;

    if (layoutModeTag == 0) {
      layout = Layout.paginated(
        pageWidth: reader.slabF32(lmPos),
        pageHeight: reader.slabF32(lmPos + 4),
        pageMarginTop: reader.slabF32(lmPos + 8),
        pageMarginBottom: reader.slabF32(lmPos + 12),
        pageMarginLeft: reader.slabF32(lmPos + 16),
        pageMarginRight: reader.slabF32(lmPos + 20),
      );
    } else {
      layout = Layout.continuous(maxWidth: reader.slabF32(lmPos));
    }

    controller.updateState(
      (state) => state.copyWith(
        settings: state.settings.copyWith(paragraphIndent: paragraphIndent, blockGap: blockGap),
        layout: layout,
      ),
    );
  }

  static void _handlePagesChanged(EditorController controller, SlateReader reader) {
    final pagesCount = reader.getU32('pages_count');
    final pagesOffset = reader.getU32('pages_offset');
    final raw = reader.readF32List(pagesOffset, pagesCount * 2);
    final pages = [for (var i = 0; i < pagesCount; i++) PageSize(width: raw[i * 2], height: raw[i * 2 + 1])];

    final hadPages = controller.state.pages.isNotEmpty;

    controller.updateState((state) => state.copyWith(pages: pages, renderVersion: Object()));

    if (!hadPages && pages.isNotEmpty) {
      controller.onEditorReady?.call();
    }
  }

  static void _handleCursorChanged(EditorController controller, SlateReader reader) {
    final pageIdx = reader.getI32('cursor_page_idx');
    if (pageIdx < 0) {
      controller.updateState((state) => state.copyWith(cursor: null));
      return;
    }

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

    final anchorPageIdx = reader.getI32('selection_anchor_page_idx');
    final anchorBounds = anchorPageIdx < 0
        ? null
        : SelectionEndpointBounds(
            pageIdx: anchorPageIdx,
            x: reader.getF32('selection_anchor_x'),
            y: reader.getF32('selection_anchor_y'),
            width: reader.getF32('selection_anchor_width'),
            height: reader.getF32('selection_anchor_height'),
          );

    final headPageIdx = reader.getI32('selection_head_page_idx');
    final headBounds = headPageIdx < 0
        ? null
        : SelectionEndpointBounds(
            pageIdx: headPageIdx,
            x: reader.getF32('selection_head_x'),
            y: reader.getF32('selection_head_y'),
            width: reader.getF32('selection_head_width'),
            height: reader.getF32('selection_head_height'),
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

    final expandable = reader.getU32('selection_expandable');

    controller.updateState(
      (state) => state.copyWith(
        selection: EditorSelection(
          collapsed: collapsed,
          cmp: cmp,
          anchorBounds: anchorBounds,
          headBounds: headBounds,
          range: {'anchor': anchor, 'head': head},
          expandable: expandable,
        ),
      ),
    );
    _updateFloatingSelection(controller, reader);

    final currentBlockNodeId = reader.readCurrentBlockNodeId();
    final isZero = currentBlockNodeId.replaceAll('0', '').isEmpty;
    controller.updateState((state) => state.copyWith(currentBlockNodeId: isZero ? null : currentBlockNodeId));

    controller.onSelectionChanged?.call(anchor, head);
  }

  static void _handleAttrsChanged(EditorController controller, SlateReader reader) {
    final attrs = reader.readAttrs();
    controller.updateState((state) => state.copyWith(attrs: attrs));
  }

  static void _updateFloatingSelection(EditorController controller, SlateReader reader) {
    final selectedBlockIds = reader.selectionBlockIds;
    final selectedBlockTypes = reader.selectionBlockTypes;
    final commonAncestorIds = reader.selectionCommonAncestorIds;
    final commonAncestorTypes = reader.selectionCommonAncestorTypes;
    final anchorNodeId = reader.readNodeIdField('selection_anchor_node_id');
    final headNodeId = reader.readNodeIdField('selection_head_node_id');
    final anchorOffset = reader.getU32('selection_anchor_offset');
    final headOffset = reader.getU32('selection_head_offset');
    final isSingleBlockRange = anchorNodeId == headNodeId && (anchorOffset - headOffset).abs() == 1;
    final resolved = _resolveFloatingSelection(
      selectedBlockIds,
      selectedBlockTypes,
      commonAncestorIds,
      commonAncestorTypes,
      canResolveSelectedContext: isSingleBlockRange,
    );
    controller.setFloatingSelection(context: resolved.context, nodeId: resolved.nodeId);
  }

  static ({String? context, String? nodeId}) _resolveFloatingSelection(
    List<String> selectedBlockIds,
    List<int> selectedBlockTypes,
    List<String> commonAncestorIds,
    List<int> commonAncestorTypes, {
    required bool canResolveSelectedContext,
  }) {
    int? selectedType;
    String? selectedNodeId;
    var hasMixedSelected = false;
    for (var i = 0; i < selectedBlockIds.length; i++) {
      final type = selectedBlockTypes[i];
      if (type == selectionTypeNone) {
        continue;
      }

      final nodeId = selectedBlockIds[i];
      if (selectedType == null) {
        selectedType = type;
        selectedNodeId = nodeId;
        continue;
      }

      if (selectedType != type || selectedNodeId != nodeId) {
        hasMixedSelected = true;
        break;
      }
    }

    if (canResolveSelectedContext && selectedType != null && selectedNodeId != null && !hasMixedSelected) {
      final selectedContext = switch (selectedType) {
        selectionTypeHorizontalRule => _selectedHorizontalRuleContext,
        selectionTypeImage => _selectedImageContext,
        selectionTypeFile => _selectedFileContext,
        selectionTypeEmbed => _selectedEmbedContext,
        selectionTypeArchived => _selectedArchivedContext,
        selectionTypeTable => _selectedTableContext,
        _ => null,
      };

      if (selectedContext != null) {
        return (context: selectedContext, nodeId: selectedNodeId);
      }
    }

    for (var i = 0; i < commonAncestorTypes.length; i++) {
      final nodeType = commonAncestorTypes[i];
      final inContext = switch (nodeType) {
        selectionTypeBulletList => _inBulletListContext,
        selectionTypeOrderedList => _inOrderedListContext,
        selectionTypeBlockquote => _inBlockquoteContext,
        selectionTypeCallout => _inCalloutContext,
        selectionTypeFold => _inFoldContext,
        selectionTypeTable => _inTableContext,
        _ => null,
      };
      if (inContext != null) {
        return (context: inContext, nodeId: commonAncestorIds[i]);
      }
    }

    return (context: null, nodeId: null);
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
    final rangesByGroup = <int, Map<String, TrackedItemRange>>{};

    for (final item in rawItems) {
      rangesByGroup.putIfAbsent(item.group, () => <String, TrackedItemRange>{})[item.id] = TrackedItemRange(
        nodeId: item.nodeId,
        startOffset: item.startOffset,
        endOffset: item.endOffset,
      );

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
        searchOverlays.add(
          SearchOverlayInfo(
            pageIdx: item.pageIdx,
            isCurrent: searchOverlays.length == controller.state.search.currentIndex,
            bounds: rects,
          ),
        );
      }
    }

    controller
      ..updateState(
        (state) => state.copyWith(
          spellcheck: SpellcheckState(overlays: spellcheckOverlays),
          aiFeedback: AiFeedbackState(overlays: aiFeedbackOverlays),
          search: state.search.copyWith(overlays: searchOverlays),
        ),
      )
      ..setTrackedItemRanges(rangesByGroup);
  }

  static void _handleTableOverlaysChanged(EditorController controller, SlateReader reader) {
    final rawOverlays = reader.readTableOverlays();
    final overlays = rawOverlays
        .map(
          (o) => TableOverlayInfo(
            pageIdx: o.pageIdx,
            tableId: o.tableId,
            bounds: TableOverlayBounds(x: o.x, y: o.y, width: o.width, height: o.height),
            borderStyle: o.borderStyle,
            align: o.align,
            proportion: o.proportion,
            startRowIndex: o.startRowIndex,
            totalRows: o.totalRows,
            isFocused: o.isFocused,
            showCellSelector: o.showCellSelector,
            contentWidth: o.contentWidth,
            minProportionWidth: o.minProportionWidth,
            maxProportionWidth: o.maxProportionWidth,
            colWidthsAsPx: o.colWidthsAsPx,
            colWidths: o.colWidths,
            colPositions: o.colPositions,
            rowHeights: o.rowHeights,
            rowPositions: o.rowPositions,
          ),
        )
        .toList();
    controller.setTableOverlays(overlays);
  }

  static void _handleFontRequired(EditorController controller, SlateReader reader) {
    final manager = controller.fontManager;
    if (manager == null) {
      return;
    }

    final requests = reader.readFontRequests();
    for (final req in requests) {
      final font = manager.findFont(req.family, req.weight);
      if (font == null) {
        continue;
      }

      unawaited(
        Future.wait([
              manager.ensureRequiredFont(req.family, font, req.codepoints).then((_) {
                unawaited(manager.preloadRemainingChunks(req.family, font));
              }),
              manager.filterUncoveredCodepoints(font, req.codepoints).then((uncovered) async {
                if (uncovered.isNotEmpty) {
                  await manager.ensureRequiredFallbackFont(req.weight, uncovered);
                }
              }),
            ])
            .then((_) {
              if (controller.isDisposed) {
                return;
              }
              controller.dispatch({
                'type': 'fontsLoaded',
                'family': req.family,
                'weight': req.weight,
                'codepoints': req.codepoints,
              });
            })
            .catchError((Object err) {
              debugPrint('Font load handler skipped: $err');
            }),
      );
    }
  }

  static void _handleExitedDocumentStart(EditorController controller) {
    controller.onExitedDocumentStart?.call();
  }

  static void _handleRepaste(EditorController controller, SlateReader reader) {
    final repaste = reader.readRepaste();
    controller.updateState((state) => state.copyWith(repasteAsTextEnabled: repaste.enabled));
  }

  static void _handleRemarksChanged(EditorController controller, SlateReader reader) {
    final rawRemarks = reader.readRemarks();
    final remarks = rawRemarks
        .map(
          (r) => RemarkOverlayInfo(
            pageIdx: r.pageIdx,
            nodeId: r.nodeId,
            remarkId: r.remarkId,
            userId: r.userId,
            text: r.text,
            createdAt: r.createdAt,
            boundsX: r.boundsX,
            boundsY: r.boundsY,
            boundsWidth: r.boundsWidth,
            boundsHeight: r.boundsHeight,
          ),
        )
        .toList();
    controller.updateState((state) => state.copyWith(remarks: remarks));
  }
}
