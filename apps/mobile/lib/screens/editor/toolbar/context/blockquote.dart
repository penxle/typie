import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/screens/editor/toolbar/buttons/custom.dart';
import 'package:typie/screens/editor/toolbar/context/node.dart';
import 'package:typie/screens/editor/values.dart';

class BlockquoteToolbar extends HookWidget {
  const BlockquoteToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = EditorStateScope.of(context);
    final proseMirrorState = useValueListenable(scope.proseMirrorState);

    final activeValue =
        proseMirrorState?.getNodeAttributes('blockquote')?['type'] as String? ?? editorDefaultValues['blockquote'];

    final keys = useMemoized(() => List.generate(editorValues['blockquote']!.length, (_) => GlobalKey()), []);

    useEffect(() {
      WidgetsBinding.instance.addPostFrameCallback((_) async {
        final index = editorValues['blockquote']!.indexWhere((e) => e['type'] == activeValue);
        if (index != -1 && keys[index].currentContext != null) {
          await Scrollable.ensureVisible(
            keys[index].currentContext!,
            alignment: 0.45,
            duration: const Duration(milliseconds: 150),
          );
        }
      });
      return null;
    }, [activeValue]);

    return NodeToolbar(
      withDelete: false,
      children: [
        Padding(
          padding: const Pad(vertical: 8),
          child: Row(
            spacing: 4,
            children: editorValues['blockquote']!.asMap().entries.map((entry) {
              final index = entry.key;
              final item = entry.value;
              final isActive = item['type'] == activeValue;

              return KeyedSubtree(
                key: keys[index],
                child: Center(
                  child: CustomToolbarButton(
                    widget: item['component'] as Widget,
                    isActive: isActive,
                    onTap: () async {
                      await scope.command('blockquote', attrs: {'blockquote': item['type']});
                    },
                  ),
                ),
              );
            }).toList(),
          ),
        ),
      ],
    );
  }
}
