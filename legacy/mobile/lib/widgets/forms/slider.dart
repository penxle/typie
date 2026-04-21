import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/widgets/forms/field.dart';

class HookFormSlider extends HookWidget {
  const HookFormSlider({
    required this.name,
    required this.min,
    required this.max,
    this.initialValue,
    this.step,
    super.key,
  });

  final String name;
  final double min;
  final double max;
  final double? initialValue;
  final double? step;

  @override
  Widget build(BuildContext context) {
    final isDragging = useState(false);

    return HookFormField<double>(
      name: name,
      initialValue: initialValue ?? min,
      builder: (context, field) {
        final value = field.value ?? min;

        return LayoutBuilder(
          builder: (context, constraints) {
            final width = constraints.maxWidth;
            final normalizedValue = (value - min) / (max - min);
            final thumbPosition = normalizedValue * width;

            void updateValue(double localX) {
              final clampedX = localX.clamp(0.0, width);
              final newNormalizedValue = clampedX / width;
              var newValue = min + (newNormalizedValue * (max - min));

              if (step != null && step! > 0) {
                newValue = ((newValue - min) / step!).round() * step! + min;
              }

              field.value = newValue;
            }

            return SizedBox(
              height: 20,
              child: GestureDetector(
                onHorizontalDragStart: (details) {
                  isDragging.value = true;
                  updateValue(details.localPosition.dx);
                },
                onHorizontalDragUpdate: (details) {
                  updateValue(details.localPosition.dx);
                },
                onHorizontalDragEnd: (_) {
                  isDragging.value = false;
                },
                onTapDown: (details) {
                  updateValue(details.localPosition.dx);
                },
                child: Stack(
                  alignment: Alignment.center,
                  clipBehavior: Clip.none,
                  children: [
                    Container(
                      height: 4,
                      decoration: BoxDecoration(
                        color: context.colors.surfaceMuted,
                        borderRadius: BorderRadius.circular(2),
                      ),
                    ),
                    Positioned(
                      left: 0,
                      child: Container(
                        height: 4,
                        width: thumbPosition,
                        decoration: BoxDecoration(
                          color: context.colors.borderStrong,
                          borderRadius: BorderRadius.circular(2),
                        ),
                      ),
                    ),
                    Positioned(
                      left: thumbPosition - 10,
                      child: AnimatedContainer(
                        duration: const Duration(milliseconds: 100),
                        width: 20,
                        height: 20,
                        decoration: BoxDecoration(
                          color: context.colors.surfaceDefault,
                          shape: BoxShape.circle,
                          border: Border.all(color: context.colors.borderStrong, width: 2),
                        ),
                      ),
                    ),
                  ],
                ),
              ),
            );
          },
        );
      },
    );
  }
}
