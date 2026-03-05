import 'dart:async';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/native_editor/state/controller.dart';
import 'package:typie/screens/native_editor/state/state.dart';
import 'package:typie/widgets/tappable.dart';

class FindReplaceSheet extends HookWidget {
  const FindReplaceSheet({required this.controller, super.key});

  final EditorController controller;

  @override
  Widget build(BuildContext context) {
    final bottomPadding = MediaQuery.paddingOf(context).bottom;

    final findTextController = useTextEditingController();
    final replaceTextController = useTextEditingController();

    final findText = useListenableSelector(findTextController, () => findTextController.text);

    final findFocusNode = useFocusNode();
    final replaceFocusNode = useFocusNode();

    final findHasFocus = useListenableSelector(findFocusNode, () => findFocusNode.hasFocus);
    final replaceHasFocus = useListenableSelector(replaceFocusNode, () => replaceFocusNode.hasFocus);

    final debounceTimer = useRef<Timer?>(null);

    final searchMatches = useRef<List<Map<String, dynamic>>>([]);
    final activeIndex = useState(0);
    final matchCount = useState(0);

    useEffect(() {
      void ensureKeyboardVisible() {
        if (findFocusNode.hasFocus || replaceFocusNode.hasFocus) {
          WidgetsBinding.instance.addPostFrameCallback((_) {
            unawaited(SystemChannels.textInput.invokeMethod<void>('TextInput.show'));
          });
        }
      }

      findFocusNode.addListener(ensureKeyboardVisible);
      replaceFocusNode.addListener(ensureKeyboardVisible);
      return () {
        findFocusNode.removeListener(ensureKeyboardVisible);
        replaceFocusNode.removeListener(ensureKeyboardVisible);
      };
    }, []);

    void updateSearchActiveIndex(int index) {
      controller.updateState(
        (state) => state.copyWith(
          search: state.search.copyWith(
            currentIndex: index,
            overlays: [
              for (var i = 0; i < state.search.overlays.length; i++)
                state.search.overlays[i].copyWith(isCurrent: i == index),
            ],
          ),
        ),
      );
    }

    void scrollToSearchMatch(int index) {
      final overlays = controller.state.search.overlays;
      if (index >= 0 && index < overlays.length) {
        final overlay = overlays[index];
        if (overlay.bounds.isNotEmpty) {
          final bound = overlay.bounds.first;
          controller.updateState(
            (state) => state.copyWith(
              search: state.search.copyWith(
                scrollTarget: SearchScrollTarget(
                  pageIdx: overlay.pageIdx,
                  x: bound.x,
                  y: bound.y,
                  width: bound.width,
                  height: bound.height,
                ),
              ),
            ),
          );
        }
      }
    }

    void performSearchAndUpdate(String query) {
      if (query.isEmpty) {
        searchMatches.value = [];
        activeIndex.value = 0;
        matchCount.value = 0;
        controller.editor.setTrackedItems(2, []);
        return;
      }

      final matches = controller.editor.performSearch(query, false);
      final items = <Map<String, dynamic>>[];
      for (var i = 0; i < matches.length; i++) {
        items.add({
          'id': 'search-$i',
          'nodeId': matches[i]['nodeId'],
          'startOffset': matches[i]['startOffset'],
          'endOffset': matches[i]['endOffset'],
        });
      }

      searchMatches.value = items;
      matchCount.value = items.length;
      if (items.isNotEmpty) {
        if (activeIndex.value >= items.length) {
          activeIndex.value = 0;
        }
      } else {
        activeIndex.value = 0;
      }

      controller.editor.setTrackedItems(2, items);
    }

    useEffect(() {
      return () {
        debounceTimer.value?.cancel();
        controller.editor.setTrackedItems(2, []);
      };
    }, []);

    useEffect(() {
      debounceTimer.value?.cancel();
      if (findText.isEmpty) {
        searchMatches.value = [];
        controller.editor.setTrackedItems(2, []);
        WidgetsBinding.instance.addPostFrameCallback((_) {
          matchCount.value = 0;
          activeIndex.value = 0;
        });
        return null;
      }
      debounceTimer.value = Timer(const Duration(milliseconds: 150), () {
        performSearchAndUpdate(findText);
        updateSearchActiveIndex(activeIndex.value);
        WidgetsBinding.instance.addPostFrameCallback((_) {
          scrollToSearchMatch(activeIndex.value);
        });
      });
      return null;
    }, [findText]);

    void findNext() {
      if (searchMatches.value.isEmpty) {
        return;
      }
      activeIndex.value = (activeIndex.value + 1) % searchMatches.value.length;
      updateSearchActiveIndex(activeIndex.value);
      scrollToSearchMatch(activeIndex.value);
    }

    void findPrevious() {
      if (searchMatches.value.isEmpty) {
        return;
      }
      activeIndex.value = activeIndex.value <= 0 ? searchMatches.value.length - 1 : activeIndex.value - 1;
      updateSearchActiveIndex(activeIndex.value);
      scrollToSearchMatch(activeIndex.value);
    }

    void replace() {
      if (controller.restrictedText) {
        controller.onEditBlocked?.call('restrictedText');
        return;
      }
      if (searchMatches.value.isEmpty) {
        return;
      }
      final match = searchMatches.value[activeIndex.value];
      controller.editor.replaceTextInBlock(
        match['nodeId'] as String,
        match['startOffset'] as int,
        match['endOffset'] as int,
        replaceTextController.text,
      );
      performSearchAndUpdate(findText);
      updateSearchActiveIndex(activeIndex.value);
      WidgetsBinding.instance.addPostFrameCallback((_) {
        scrollToSearchMatch(activeIndex.value);
      });
    }

    void replaceAll() {
      if (controller.restrictedText) {
        controller.onEditBlocked?.call('restrictedText');
        return;
      }
      if (searchMatches.value.isEmpty) {
        return;
      }
      final items = searchMatches.value
          .map((m) => [m['nodeId'], m['startOffset'], m['endOffset'], replaceTextController.text])
          .toList();
      controller.editor.replaceTextInBlocks(items);
      performSearchAndUpdate(findText);
    }

    final totalCount = matchCount.value;
    final currentIndex = activeIndex.value;

    return AppBottomSheet(
      includeBottomPadding: false,
      padding: Pad(left: 20, top: 16, right: 20, bottom: bottomPadding + 12),
      child: Row(
        spacing: 12,
        children: [
          Expanded(
            child: Column(
              spacing: 12,
              children: [
                Container(
                  height: 44,
                  decoration: BoxDecoration(
                    border: Border.all(
                      color: findHasFocus ? context.colors.borderStrong : context.colors.borderDefault,
                    ),
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Padding(
                    padding: const Pad(horizontal: 16),
                    child: Row(
                      children: [
                        Expanded(
                          child: TextField(
                            controller: findTextController,
                            focusNode: findFocusNode,
                            decoration: InputDecoration.collapsed(
                              hintText: '찾기',
                              hintStyle: TextStyle(
                                fontSize: 16,
                                fontWeight: FontWeight.w500,
                                color: context.colors.textDisabled,
                              ),
                            ),
                            style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w500),
                            cursorColor: context.colors.textDefault,
                            autofocus: true,
                            textInputAction: TextInputAction.search,
                            onEditingComplete: findNext,
                          ),
                        ),
                        if (findText.isNotEmpty)
                          Text(
                            totalCount > 0 ? '${currentIndex + 1} / $totalCount' : '결과 없음',
                            style: TextStyle(fontSize: 14, color: context.colors.textSubtle),
                          ),
                      ],
                    ),
                  ),
                ),
                Container(
                  height: 44,
                  decoration: BoxDecoration(
                    border: Border.all(
                      color: replaceHasFocus ? context.colors.borderStrong : context.colors.borderDefault,
                    ),
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Center(
                    child: Padding(
                      padding: const Pad(horizontal: 16),
                      child: TextField(
                        controller: replaceTextController,
                        focusNode: replaceFocusNode,
                        decoration: InputDecoration.collapsed(
                          hintText: '바꾸기',
                          hintStyle: TextStyle(
                            fontSize: 16,
                            fontWeight: FontWeight.w500,
                            color: context.colors.textDisabled,
                          ),
                        ),
                        style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w500),
                        cursorColor: context.colors.textDefault,
                        textInputAction: TextInputAction.search,
                        onEditingComplete: replace,
                      ),
                    ),
                  ),
                ),
              ],
            ),
          ),
          Column(
            spacing: 12,
            children: [
              SizedBox(
                height: 44,
                child: Row(
                  spacing: 8,
                  children: [
                    _ActionButton(icon: LucideLightIcons.arrow_up, enabled: findText.isNotEmpty, onTap: findPrevious),
                    _ActionButton(icon: LucideLightIcons.arrow_down, enabled: findText.isNotEmpty, onTap: findNext),
                  ],
                ),
              ),
              SizedBox(
                height: 44,
                child: Row(
                  spacing: 8,
                  children: [
                    _ActionButton(icon: LucideLightIcons.replace, enabled: findText.isNotEmpty, onTap: replace),
                    _ActionButton(icon: LucideLightIcons.replace_all, enabled: findText.isNotEmpty, onTap: replaceAll),
                  ],
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

class _ActionButton extends StatelessWidget {
  const _ActionButton({required this.icon, required this.enabled, required this.onTap});

  final IconData icon;
  final bool enabled;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return Tappable(
      onTap: onTap,
      child: Container(
        width: 36,
        height: 36,
        decoration: BoxDecoration(
          border: Border.all(color: enabled ? context.colors.borderStrong : context.colors.borderDefault),
          borderRadius: BorderRadius.circular(6),
        ),
        child: Center(
          child: Icon(icon, size: 18, color: enabled ? context.colors.textDefault : context.colors.textFaint),
        ),
      ),
    );
  }
}
