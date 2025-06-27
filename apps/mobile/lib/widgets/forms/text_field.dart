import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/hooks/async_effect.dart';
import 'package:typie/widgets/forms/field.dart';
import 'package:typie/widgets/tappable.dart';

class HookFormTextField extends HookWidget {
  const HookFormTextField({
    required this.name,
    required this.label,
    required this.placeholder,
    super.key,
    this.controller,
    this.focusNode,
    this.autofocus = false,
    this.obscureText = false,
    this.keyboardType,
    this.textInputAction = TextInputAction.done,
    this.autofillHints,
    this.initialValue,
    this.submitOnEnter = true,
  });

  const factory HookFormTextField.collapsed({
    required String name,
    required TextStyle style,
    required String placeholder,
    TextEditingController? controller,
    FocusNode? focusNode,
    bool autofocus,
    String? initialValue,
    TextInputType? keyboardType,
    TextInputAction textInputAction,
    bool submitOnEnter,
    Key? key,
  }) = _HookFormCollapsedTextField;

  final String name;
  final TextEditingController? controller;
  final FocusNode? focusNode;
  final String label;
  final String placeholder;
  final bool autofocus;
  final bool obscureText;
  final TextInputType? keyboardType;
  final TextInputAction textInputAction;
  final List<String>? autofillHints;
  final String? initialValue;
  final bool submitOnEnter;

  @override
  Widget build(BuildContext context) {
    final builtinController = useTextEditingController();
    final builtinFocusNode = useFocusNode();

    final effectiveController = controller ?? builtinController;
    final effectiveFocusNode = focusNode ?? builtinFocusNode;

    final animationController = useAnimationController(duration: const Duration(milliseconds: 150));
    final curve = useMemoized(() => CurvedAnimation(parent: animationController, curve: Curves.ease));
    final colorTween = useRef<ColorTween?>(null);

    final defaultColor = context.colors.borderInput;

    useEffect(() {
      if (initialValue != null) {
        effectiveController.text = initialValue!;
      }

      return null;
    }, []);

    useAsyncEffect(() async {
      if (autofocus) {
        await ModalRoute.of(context)?.didPush();
        effectiveFocusNode.requestFocus();
      }

      return null;
    }, [autofocus]);

    return HookFormField(
      name: name,
      initialValue: initialValue,
      builder: (context, field) {
        final hasFocus = useListenableSelector(effectiveFocusNode, () => effectiveFocusNode.hasFocus);

        final focusedColor = context.colors.borderStrong;

        useEffect(() {
          final begin = colorTween.value?.evaluate(curve);
          final end = hasFocus ? focusedColor : defaultColor;

          colorTween.value = ColorTween(begin: begin ?? end, end: end);
          animationController.forward(from: 0);

          return null;
        }, [hasFocus]);

        return AnimatedBuilder(
          animation: animationController,
          builder: (context, child) {
            final color = colorTween.value?.evaluate(curve) ?? defaultColor;

            return Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Tappable(
                  onTap: effectiveFocusNode.requestFocus,
                  child: Row(
                    spacing: 8,
                    children: [
                      Expanded(
                        child: Text(label, style: const TextStyle(fontSize: 14, fontWeight: FontWeight.w500)),
                      ),
                      if (field.error != null)
                        Text(
                          field.error!,
                          style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500, color: context.colors.textDanger),
                        ),
                    ],
                  ),
                ),
                const Gap(4),
                Container(
                  decoration: BoxDecoration(
                    border: Border.all(color: color),
                    borderRadius: BorderRadius.circular(8),
                  ),
                  padding: const Pad(horizontal: 16, vertical: 12),
                  child: child,
                ),
              ],
            );
          },
          child: TextField(
            controller: controller ?? builtinController,
            focusNode: focusNode ?? builtinFocusNode,
            smartDashesType: SmartDashesType.disabled,
            smartQuotesType: SmartQuotesType.disabled,
            obscureText: obscureText,
            keyboardType: keyboardType,
            textInputAction: textInputAction,
            autofillHints: autofillHints,
            decoration: InputDecoration.collapsed(
              hintText: placeholder,
              hintStyle: TextStyle(fontSize: 16, fontWeight: FontWeight.w500, color: context.colors.textPlaceholder),
            ),
            cursorColor: context.colors.textDefault,
            style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w500),
            onChanged: (value) {
              field.value = value;
            },
            onSubmitted: (value) async {
              if (submitOnEnter && textInputAction == TextInputAction.done) {
                await field.form.submit();
              }
            },
          ),
        );
      },
    );
  }
}

class _HookFormCollapsedTextField extends HookFormTextField {
  const _HookFormCollapsedTextField({
    required super.name,
    required this.style,
    required super.placeholder,
    super.controller,
    super.focusNode,
    super.autofocus,
    super.initialValue,
    super.keyboardType,
    super.textInputAction = TextInputAction.done,
    super.submitOnEnter = true,
    super.key,
  }) : super(label: '');

  final TextStyle style;

  @override
  Widget build(BuildContext context) {
    final builtinController = useTextEditingController();
    final builtinFocusNode = useFocusNode();

    final effectiveController = controller ?? builtinController;
    final effectiveFocusNode = focusNode ?? builtinFocusNode;

    useAsyncEffect(() async {
      if (autofocus) {
        await ModalRoute.of(context)!.didPush();
        effectiveFocusNode.requestFocus();
      }

      return null;
    }, [autofocus]);

    useEffect(() {
      if (initialValue != null) {
        effectiveController.text = initialValue!;
      }

      return null;
    }, [initialValue]);

    return HookFormField(
      name: name,
      initialValue: initialValue,
      builder: (context, field) {
        return TextField(
          controller: effectiveController,
          focusNode: effectiveFocusNode,
          smartDashesType: SmartDashesType.disabled,
          smartQuotesType: SmartQuotesType.disabled,
          obscureText: obscureText,
          keyboardType: keyboardType,
          textInputAction: textInputAction,
          style: style,
          decoration: InputDecoration.collapsed(
            hintText: placeholder,
            hintStyle: style.copyWith(color: context.colors.textPlaceholder),
          ),
          onChanged: (value) {
            field.value = value;
          },
          onSubmitted: (value) async {
            if (submitOnEnter && textInputAction == TextInputAction.done) {
              await field.form.submit();
            }
          },
        );
      },
    );
  }
}
