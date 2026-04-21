import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/widgets/settings_screen.dart';

@RoutePage()
class OssLicensesScreen extends HookWidget {
  const OssLicensesScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final future = useMemoized(() {
      return LicenseRegistry.licenses.fold(<LicenseEntry>[], (prev, license) => prev..add(license)).then((entries) {
        final licenses = <String, List<String>>{};
        for (final entry in entries) {
          for (final package in entry.packages) {
            licenses.putIfAbsent(package, () => []).addAll(entry.paragraphs.map((v) => v.text));
          }
        }
        return licenses.entries.toList()..sort((a, b) => a.key.compareTo(b.key));
      });
    });

    final licenses = useFuture(future);
    final scrollController = useScrollController();

    return SettingsOverlayScreen(
      title: '오픈소스 라이센스',
      scrollController: scrollController,
      bodyBuilder: (context, title, padding) {
        final itemCount = licenses.hasData ? licenses.data!.length + 1 : 2;

        return ListView.separated(
          controller: scrollController,
          physics: const AlwaysScrollableScrollPhysics(),
          padding: padding,
          itemCount: itemCount,
          itemBuilder: (context, index) {
            if (index == 0) {
              return Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  title,
                  const Gap(settingsSectionGap),
                  const SettingsSectionLabel(text: '패키지', top: 0),
                ],
              );
            }

            if (!licenses.hasData) {
              return const SettingsSectionCard(
                child: SizedBox(height: 220, child: Center(child: CircularProgressIndicator())),
              );
            }

            final license = licenses.data![index - 1];

            return SettingsSectionCard(
              clipBehavior: Clip.antiAlias,
              child: SettingsListRow(
                label: license.key,
                onTap: () async {
                  await context.showBottomSheet(
                    child: AppFullBottomSheet(
                      title: license.key,
                      padding: Pad.zero,
                      child: ListView.separated(
                        physics: const AlwaysScrollableScrollPhysics(),
                        padding: Pad(all: 20, bottom: MediaQuery.paddingOf(context).bottom),
                        itemCount: license.value.length,
                        itemBuilder: (context, index) {
                          return Text(license.value[index], style: const TextStyle(fontSize: 14));
                        },
                        separatorBuilder: (context, index) {
                          return const Gap(12);
                        },
                      ),
                    ),
                  );
                },
              ),
            );
          },
          separatorBuilder: (context, index) {
            if (index == 0) {
              return const SizedBox.shrink();
            }

            return const Gap(12);
          },
        );
      },
    );
  }
}
