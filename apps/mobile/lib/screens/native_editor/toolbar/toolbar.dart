import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/screens/native_editor/toolbar/bottom/bottom.dart';
import 'package:typie/screens/native_editor/toolbar/primary/primary.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/screens/native_editor/toolbar/secondary/secondary.dart';

class NativeEditorToolbar extends HookWidget {
  const NativeEditorToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = NativeEditorToolbarScope.of(context);
    final isKeyboardVisible = useValueListenable(scope.isKeyboardVisible);
    final keyboardHeight = useValueListenable(scope.keyboardHeight);
    final isEditorFocused = useValueListenable(scope.isEditorFocused);

    if (!isEditorFocused) {
      if (isKeyboardVisible) {
        return SizedBox(height: keyboardHeight);
      }
      return const SizedBox.shrink();
    }

    return const Column(
      children: [NativeEditorSecondaryToolbar(), NativeEditorPrimaryToolbar(), NativeEditorBottomToolbar()],
    );
  }
}
