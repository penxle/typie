import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:collection/collection.dart';
import 'package:flutter/material.dart';
import 'package:gap/gap.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/widgets/forms/field.dart';
import 'package:typie/widgets/tappable.dart';

class HookFormSelect<T> extends StatelessWidget {
  const HookFormSelect({required this.name, required this.initialValue, required this.items, this.values, super.key});

  final String name;
  final T initialValue;
  final List<T>? values;
  final List<HookFormSelectItem<T>> items;

  @override
  Widget build(BuildContext context) {
    return HookFormField(
      name: name,
      initialValue: initialValue,
      builder: (context, field) {
        final selectedValues = values ?? [field.value as T];
        final isIndeterminate = selectedValues.toSet().length > 1;

        final item = isIndeterminate
            ? HookFormSelectItem<T>(
                icon: LucideLightIcons.minus,
                label: selectedValues
                    .map((v) => items.firstWhereOrNull((item) => item.value == v)?.label ?? '')
                    .join(', '),
                value: field.value as T,
              )
            : items.firstWhereOrNull((item) => item.value == field.value);

        return Tappable(
          onTap: () async {
            await context.showBottomSheet(
              child: AppBottomSheet(
                padding: const Pad(horizontal: 20, bottom: 8),
                child: Column(
                  spacing: 8,
                  children: [
                    ...items.map(
                      (item) => Tappable(
                        onTap: () async {
                          field.value = item.value;
                          await context.router.root.maybePop();
                        },
                        child: Container(
                          decoration: BoxDecoration(
                            border: Border.all(
                              color: selectedValues.contains(item.value)
                                  ? context.colors.borderInverse
                                  : context.colors.borderDefault,
                            ),
                            borderRadius: BorderRadius.circular(8),
                          ),
                          padding: const Pad(horizontal: 12, vertical: 12),
                          child: Row(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            spacing: 8,
                            children: [
                              if (item.icon != null)
                                Padding(padding: const Pad(top: 2), child: Icon(item.icon, size: 18)),
                              Column(
                                crossAxisAlignment: CrossAxisAlignment.start,
                                spacing: 2,
                                children: [
                                  Text(item.label, style: const TextStyle(fontSize: 16)),
                                  if (item.description != null)
                                    Text(
                                      item.description!,
                                      style: TextStyle(fontSize: 15, color: context.colors.textFaint),
                                    ),
                                ],
                              ),
                            ],
                          ),
                        ),
                      ),
                    ),
                  ],
                ),
              ),
            );
          },
          child: Row(
            children: [
              if (item?.icon != null) ...[Icon(item!.icon, size: 18), const Gap(4)],
              Text(item?.label ?? '(알 수 없음)', style: const TextStyle(fontSize: 16)),
              const Gap(8),
              const Icon(LucideLightIcons.chevron_down, size: 16),
            ],
          ),
        );
      },
    );
  }
}

class HookFormSelectItem<T> {
  const HookFormSelectItem({required this.label, required this.value, this.icon, this.description});

  final IconData? icon;
  final String label;
  final String? description;
  final T value;
}
