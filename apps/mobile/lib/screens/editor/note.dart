import 'dart:async';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:back_button_interceptor/back_button_interceptor.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/hooks/async_effect.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/tappable.dart';

class Note extends HookWidget {
  const Note({super.key, required this.onBack});

  final void Function() onBack;

  @override
  Widget build(BuildContext context) {
    useAutomaticKeepAlive();

    final controller = useTextEditingController();
    final focusNode = useFocusNode();

    final scope = EditorStateScope.of(context);
    final yjsState = useValueListenable(scope.yjsState);
    final mode = useValueListenable(scope.mode);

    final debounceTimer = useRef<Timer?>(null);

    useEffect(() {
      if (controller.text.isEmpty) {
        controller.value = TextEditingValue(
          text: yjsState?.note ?? '',
          selection: const TextSelection.collapsed(offset: 0),
        );
      } else {
        if (controller.text != yjsState?.note) {
          controller.text = yjsState?.note ?? '';
        }
      }

      return null;
    }, [yjsState?.note]);

    useEffect(() {
      if (mode != EditorMode.note) {
        return null;
      }

      bool handler(bool stopDefaultButtonEvent, RouteInfo routeInfo) {
        onBack();
        return true;
      }

      BackButtonInterceptor.add(handler);

      return () {
        BackButtonInterceptor.remove(handler);
      };
    }, [mode]);

    useAsyncEffect(() async {
      if (mode == EditorMode.note && scope.isKeyboardVisible.value) {
        await Future<void>.delayed(const Duration(milliseconds: 500));
        focusNode.requestFocus();
      }

      return null;
    }, [mode]);

    useEffect(() {
      return () {
        debounceTimer.value?.cancel();
      };
    }, []);

    return Screen(
      resizeToAvoidBottomInset: true,
      heading: Heading(
        leadingWidget: Tappable(
          onTap: onBack,
          padding: const Pad(vertical: 4),
          child: SizedBox(width: 52, child: Icon(LucideLightIcons.chevron_left, color: context.colors.textDefault)),
        ),
        titleIcon: LucideLightIcons.notebook_tabs,
        title: '작성 노트',
        backgroundColor: context.colors.surfaceDefault,
      ),
      backgroundColor: context.colors.surfaceDefault,
      child: LayoutBuilder(
        builder: (context, constraints) {
          return SingleChildScrollView(
            physics: const AlwaysScrollableScrollPhysics(),
            padding: Pad(all: 20, bottom: MediaQuery.viewPaddingOf(context).bottom),
            child: ConstrainedBox(
              constraints: BoxConstraints(minHeight: constraints.maxHeight - 40),
              child: TextField(
                controller: controller,
                focusNode: focusNode,
                smartDashesType: SmartDashesType.disabled,
                smartQuotesType: SmartQuotesType.disabled,
                autocorrect: false,
                keyboardType: TextInputType.multiline,
                maxLines: null,
                textAlignVertical: TextAlignVertical.top,
                scrollPadding: const Pad(bottom: 100),
                decoration: InputDecoration.collapsed(
                  hintText: '포스트에 대해 기억할 내용이나 작성에 도움이 되는 내용이 있다면 자유롭게 적어보세요. \n\n글쓰기 중 상단바를 쓸어넘겨서 작성 노트를 열 수 있어요.',
                  hintStyle: TextStyle(fontSize: 16, fontWeight: FontWeight.w500, color: context.colors.textDisabled),
                ),
                onChanged: (value) {
                  debounceTimer.value?.cancel();
                  debounceTimer.value = Timer(Duration.zero, () async {
                    await scope.command('note', attrs: {'note': value});
                  });
                },
              ),
            ),
          );
        },
      ),
    );
  }
}
