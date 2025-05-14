import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/services/keyboard.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/tappable.dart';
import 'package:typie/widgets/webview.dart';

class EditorToolbar extends HookWidget {
  const EditorToolbar({required this.webViewController, super.key});

  final WebViewController? webViewController;

  @override
  Widget build(BuildContext context) {
    final keyboard = useService<Keyboard>();

    final keyboardHeight = useState(keyboard.height);
    final isKeyboardVisible = useState(keyboard.isVisible);

    useEffect(() {
      final subscription = keyboard.onHeightChange.listen((height) {
        if (height > 0) {
          keyboardHeight.value = height;
        }

        isKeyboardVisible.value = height > 0;
      });

      return subscription.cancel;
    }, [keyboard.onHeightChange]);

    if (!isKeyboardVisible.value) {
      return const SizedBox.shrink();
    }

    return Column(
      children: [
        Box(
          height: 46,
          decoration: const BoxDecoration(
            color: AppColors.white,
            border: Border(top: BorderSide(color: AppColors.gray_200)),
          ),
          padding: const Pad(horizontal: 12),
          child: Row(
            spacing: 24,
            children: [
              Expanded(
                child: SingleChildScrollView(
                  scrollDirection: Axis.horizontal,
                  child: Row(
                    spacing: 24,
                    children: [
                      _ToolbarButton(icon: LucideLightIcons.bold, onTap: () {}),
                      _ToolbarButton(icon: LucideLightIcons.italic, onTap: () {}),
                      _ToolbarButton(icon: LucideLightIcons.underline, onTap: () {}),
                      _ToolbarButton(icon: LucideLightIcons.strikethrough, onTap: () {}),
                      _ToolbarButton(icon: LucideLightIcons.list, onTap: () {}),
                      _ToolbarButton(icon: LucideLightIcons.image, onTap: () {}),
                      _ToolbarButton(icon: LucideLightIcons.link, onTap: () {}),
                      _ToolbarButton(icon: LucideLightIcons.align_left, onTap: () {}),
                      _ToolbarButton(icon: LucideLightIcons.align_center, onTap: () {}),
                      _ToolbarButton(icon: LucideLightIcons.align_right, onTap: () {}),
                    ],
                  ),
                ),
              ),
              _ToolbarButton(
                icon: LucideLightIcons.keyboard_off,
                onTap: () async {
                  await webViewController?.emitEvent('blur');
                },
              ),
            ],
          ),
        ),
        Box(
          height: keyboardHeight.value,
          decoration: const BoxDecoration(
            color: AppColors.white,
            border: Border(top: BorderSide(color: AppColors.gray_200)),
          ),
        ),
      ],
    );
  }
}

class _ToolbarButton extends StatelessWidget {
  const _ToolbarButton({required this.icon, required this.onTap});

  final IconData icon;
  final void Function() onTap;

  @override
  Widget build(BuildContext context) {
    return AnimatedTappable(
      builder: (context, animation) {
        final curve = CurvedAnimation(parent: animation, curve: Curves.ease);
        final tweenedColor = ColorTween(begin: AppColors.gray_700, end: AppColors.gray_300).animate(curve);

        return AnimatedBuilder(
          animation: curve,
          builder: (context, child) {
            return Box(padding: const Pad(all: 1), child: Icon(icon, size: 22, color: tweenedColor.value));
          },
        );
      },
      onTap: onTap,
    );
  }
}
