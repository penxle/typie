import 'dart:async';
import 'dart:io';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:built_value/json_object.dart';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:gql_tristate_value/gql_tristate_value.dart';
import 'package:mixpanel_flutter/mixpanel_flutter.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/loader.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/graphql/hooks.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/screens/site/__generated__/create_site_mutation.req.gql.dart';
import 'package:typie/screens/site/__generated__/persist_blob_as_image_mutation.req.gql.dart';
import 'package:typie/screens/site/__generated__/screen_query.data.gql.dart';
import 'package:typie/screens/site/__generated__/screen_query.req.gql.dart';
import 'package:typie/screens/site/__generated__/update_site_mutation.req.gql.dart';
import 'package:typie/services/blob.dart';
import 'package:typie/services/site.dart';
import 'package:typie/widgets/horizontal_divider.dart';
import 'package:typie/widgets/img.dart';
import 'package:typie/widgets/plan_upgrade_bottom_sheet.dart';
import 'package:typie/widgets/settings_screen.dart';
import 'package:typie/widgets/tappable.dart';

@RoutePage()
class SiteScreen extends HookWidget {
  const SiteScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final site = useService<Site>();
    final siteId = useValueListenable(site);
    final data = useQuery(GSiteScreen_QueryReq((b) => b.vars.siteId = siteId));

    final scrollController = useScrollController();
    final currentSiteId = data?.site.id;
    final otherSiteCount = switch (currentSiteId) {
      final String id => data?.me?.sites.where((s) => s.id != id).length ?? 0,
      null => 0,
    };
    final showOtherSpaces = otherSiteCount > 0 || data?.me?.subscription != null;

    return SettingsOverlayScreen(
      title: '스페이스',
      scrollController: scrollController,
      loading: data == null,
      padding: EdgeInsets.fromLTRB(20, 0, 20, MediaQuery.paddingOf(context).bottom + 140),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          const Gap(settingsSectionGap),
          _CurrentSpaceHero(data: data, site: site),
          if (showOtherSpaces) ...[const Gap(settingsSectionGap), _OtherSpaces(data: data, site: site)],
        ],
      ),
    );
  }
}

class _CurrentSpaceHero extends HookWidget {
  const _CurrentSpaceHero({required this.data, required this.site});

  final GSiteScreen_QueryData? data;
  final Site site;

  @override
  Widget build(BuildContext context) {
    final client = useService<GraphQLClient>();
    final blob = useService<Blob>();
    final mixpanel = useService<Mixpanel>();

    final nameValue = useState(data?.site.name ?? '');
    final isEditing = useState(false);
    final nameController = useTextEditingController(text: nameValue.value);
    final nameFocusNode = useFocusNode();

    useEffect(() {
      if (data != null && !isEditing.value) {
        nameValue.value = data!.site.name;
        nameController.text = data!.site.name;
      }
      return null;
    }, [data?.site.id]);

    useEffect(() {
      void listener() {
        if (!nameFocusNode.hasFocus && isEditing.value) {
          unawaited(
            _submitName(
              context: context,
              client: client,
              mixpanel: mixpanel,
              siteId: data?.site.id ?? '',
              nameController: nameController,
              nameValue: nameValue,
              isEditing: isEditing,
            ),
          );
        }
      }

      nameFocusNode.addListener(listener);
      return () => nameFocusNode.removeListener(listener);
    }, [nameFocusNode, data?.site.id]);

    Future<void> updateSiteLogo() async {
      if (data == null) {
        return;
      }

      try {
        final result = await FilePicker.platform.pickFiles(type: FileType.image);
        if (result == null || result.files.isEmpty || result.files.first.path == null) {
          return;
        }

        final file = File(result.files.first.path!);
        final path = await blob.upload(file);
        final image = await client.request(
          GSiteScreen_PersistBlobAsImage_MutationReq(
            (b) => b
              ..vars.input.path = path
              ..vars.input.modification = Value.present(
                JsonObject({
                  'resize': {'width': 512, 'height': 512, 'fit': 'cover', 'withoutEnlargement': true},
                  'format': 'png',
                }),
              ),
          ),
        );

        await client.request(
          GSiteScreen_UpdateSite_MutationReq(
            (b) => b
              ..vars.input.siteId = data!.site.id
              ..vars.input.logoId = Value.present(image.persistBlobAsImage.id),
          ),
        );

        unawaited(mixpanel.track('update_site_logo', properties: {'via': 'space_screen'}));

        if (context.mounted) {
          context.toast(ToastType.success, '스페이스 로고가 변경되었어요.');
        }
      } catch (_) {
        if (context.mounted) {
          context.toast(ToastType.error, '오류가 발생했어요. 잠시 후 다시 시도해주세요.');
        }
      }
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        const SettingsSectionLabel(text: '현재 스페이스'),
        SettingsSectionCard(
          padding: const Pad(top: 24, bottom: 32, left: 24, right: 24),
          child: Column(
            children: [
              Tappable(
                onTap: updateSiteLogo,
                child: ClipRRect(
                  borderRadius: BorderRadius.circular(14),
                  child: Img(image: data?.site.logo, size: 64),
                ),
              ),
              const Gap(12),
              if (isEditing.value)
                SizedBox(
                  width: 200,
                  child: TextField(
                    controller: nameController,
                    focusNode: nameFocusNode,
                    textAlign: TextAlign.center,
                    style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w700),
                    decoration: InputDecoration(
                      contentPadding: const Pad(horizontal: 12, vertical: 8),
                      border: OutlineInputBorder(
                        borderRadius: BorderRadius.circular(8),
                        borderSide: BorderSide(color: context.colors.accentBrand),
                      ),
                      focusedBorder: OutlineInputBorder(
                        borderRadius: BorderRadius.circular(8),
                        borderSide: BorderSide(color: context.colors.accentBrand, width: 1.5),
                      ),
                      filled: true,
                      fillColor: context.colors.surfaceDefault,
                    ),
                    onSubmitted: (_) {
                      unawaited(
                        _submitName(
                          context: context,
                          client: client,
                          mixpanel: mixpanel,
                          siteId: data?.site.id ?? '',
                          nameController: nameController,
                          nameValue: nameValue,
                          isEditing: isEditing,
                        ),
                      );
                    },
                  ),
                )
              else
                Tappable(
                  onTap: () {
                    isEditing.value = true;
                    WidgetsBinding.instance.addPostFrameCallback((_) {
                      nameFocusNode.requestFocus();
                    });
                  },
                  child: Text(nameValue.value, style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w700)),
                ),
              const Gap(12),
              Tappable(
                onTap: () {
                  unawaited(context.router.push(const SiteSettingsRoute()));
                },
                child: Container(
                  padding: const Pad(horizontal: 12, vertical: 6),
                  decoration: BoxDecoration(color: context.colors.surfaceMuted, borderRadius: BorderRadius.circular(8)),
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    spacing: 4,
                    children: [
                      Icon(LucideLightIcons.settings, size: 14, color: context.colors.textSubtle),
                      Text('스페이스 설정', style: TextStyle(fontSize: 13, color: context.colors.textSubtle)),
                    ],
                  ),
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }

  Future<void> _submitName({
    required BuildContext context,
    required GraphQLClient client,
    required Mixpanel mixpanel,
    required String siteId,
    required TextEditingController nameController,
    required ValueNotifier<String> nameValue,
    required ValueNotifier<bool> isEditing,
  }) async {
    final rawName = nameController.text;
    final nextName = rawName.trim();

    isEditing.value = false;

    if (nextName.isEmpty || nextName == nameValue.value) {
      nameController.text = nameValue.value;
      return;
    }

    try {
      await client.request(
        GSiteScreen_UpdateSite_MutationReq(
          (b) => b
            ..vars.input.siteId = siteId
            ..vars.input.name = Value.present(nextName),
        ),
      );

      nameValue.value = nextName;
      nameController.text = nextName;
      unawaited(mixpanel.track('update_site_name', properties: {'via': 'space_screen'}));

      if (context.mounted) {
        context.toast(ToastType.success, '스페이스 이름이 변경되었어요.');
      }
    } catch (_) {
      nameController.text = nameValue.value;
      if (context.mounted) {
        context.toast(ToastType.error, '오류가 발생했어요. 잠시 후 다시 시도해주세요.');
      }
    }
  }
}

class _OtherSpaces extends HookWidget {
  const _OtherSpaces({required this.data, required this.site});

  final GSiteScreen_QueryData? data;
  final Site site;

  @override
  Widget build(BuildContext context) {
    final client = useService<GraphQLClient>();
    final mixpanel = useService<Mixpanel>();
    final hasSubscription = data?.me?.subscription != null;

    final otherSites = data?.me?.sites.where((s) => s.id != data?.site.id).toList() ?? [];

    if (otherSites.isEmpty && !hasSubscription) {
      return const SizedBox.shrink();
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const SettingsSectionLabel(text: '다른 스페이스'),
        SettingsSectionCard(
          clipBehavior: Clip.antiAlias,
          child: Column(
            children: [
              ...otherSites.map((s) {
                return Column(
                  children: [
                    Tappable(
                      onTap: () {
                        site.setSiteId(s.id);
                        unawaited(context.router.maybePop());
                      },
                      child: Padding(
                        padding: const Pad(horizontal: 16, vertical: 14),
                        child: Tappable.scale(
                          child: Row(
                            spacing: 12,
                            children: [
                              ClipRRect(
                                borderRadius: BorderRadius.circular(6),
                                child: Img(image: s.logo, size: 28),
                              ),
                              Expanded(
                                child: Text(
                                  s.name,
                                  style: const TextStyle(fontSize: 15, fontWeight: FontWeight.w500),
                                  overflow: TextOverflow.ellipsis,
                                ),
                              ),
                            ],
                          ),
                        ),
                      ),
                    ),
                    HorizontalDivider(color: context.colors.borderSubtle),
                  ],
                );
              }),
              Tappable(
                onTap: () async {
                  if (!hasSubscription) {
                    if (context.mounted) {
                      final result = await context.showBottomSheet<PlanUpgradeResult>(
                        child: const PlanUpgradeBottomSheet(message: '멀티 스페이스는 FULL ACCESS 플랜에서 사용할 수 있어요.'),
                      );

                      if (result == PlanUpgradeResult.upgrade && context.mounted) {
                        unawaited(context.router.push(const EnrollPlanRoute()));
                      }
                    }
                    return;
                  }

                  if (context.mounted) {
                    await context.showBottomSheet(
                      child: _CreateSiteBottomSheet(client: client, site: site, mixpanel: mixpanel),
                    );
                  }
                },
                child: Padding(
                  padding: const Pad(horizontal: 16, vertical: 14),
                  child: Tappable.scale(
                    child: Row(
                      spacing: 12,
                      children: [
                        SizedBox(
                          width: 28,
                          child: Center(child: Icon(LucideLightIcons.plus, size: 18, color: context.colors.textSubtle)),
                        ),
                        Text('새 스페이스 생성', style: TextStyle(fontSize: 15, color: context.colors.textSubtle)),
                      ],
                    ),
                  ),
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }
}

class _CreateSiteBottomSheet extends HookWidget {
  const _CreateSiteBottomSheet({required this.client, required this.site, required this.mixpanel});

  final GraphQLClient client;
  final Site site;
  final Mixpanel mixpanel;

  @override
  Widget build(BuildContext context) {
    final controller = useTextEditingController();

    return ConfirmBottomSheet(
      title: '새 스페이스 생성',
      message: '스페이스는 독립된 글쓰기 공간이에요.\n주제나 목적에 따라 글을 나누어 관리해보세요.',
      confirmText: '생성',
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        spacing: 6,
        children: [
          Text(
            '스페이스 이름',
            style: TextStyle(fontSize: 13, fontWeight: FontWeight.w500, color: context.colors.textDefault),
          ),
          TextField(
            controller: controller,
            autofocus: true,
            decoration: InputDecoration(
              hintText: '새 스페이스',
              hintStyle: TextStyle(color: context.colors.textDisabled),
              border: OutlineInputBorder(borderRadius: BorderRadius.circular(8)),
              contentPadding: const Pad(horizontal: 12, vertical: 10),
            ),
            style: const TextStyle(fontSize: 16),
          ),
        ],
      ),
      onConfirm: () async {
        final name = controller.text.trim();

        try {
          await context.runWithLoader(() async {
            final result = await client.request(
              GSiteScreen_CreateSite_MutationReq((b) => b..vars.input.name = name.isEmpty ? '새 스페이스' : name),
            );

            site.setSiteId(result.createSite.id);
          });

          unawaited(mixpanel.track('create_site', properties: {'via': 'space_screen'}));

          if (context.mounted) {
            context.toast(ToastType.success, '새 스페이스가 생성되었어요.');
          }
        } catch (_) {
          if (context.mounted) {
            context.toast(ToastType.error, '오류가 발생했어요. 잠시 후 다시 시도해주세요.');
          }
        }
      },
    );
  }
}
