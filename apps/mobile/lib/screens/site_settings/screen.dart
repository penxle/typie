import 'dart:async';
import 'dart:io';
import 'dart:math';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:built_value/json_object.dart';
import 'package:cached_network_image/cached_network_image.dart';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gql_tristate_value/gql_tristate_value.dart';
import 'package:luthor/luthor.dart';
import 'package:mixpanel_flutter/mixpanel_flutter.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/modal.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/env.dart';
import 'package:typie/graphql/__generated__/schema.schema.gql.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/graphql/error.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/screens/site_settings/__generated__/delete_site_mutation.req.gql.dart';
import 'package:typie/screens/site_settings/__generated__/persist_blob_as_image_mutation.req.gql.dart';
import 'package:typie/screens/site_settings/__generated__/screen_query.data.gql.dart';
import 'package:typie/screens/site_settings/__generated__/screen_query.req.gql.dart';
import 'package:typie/screens/site_settings/__generated__/update_site_mutation.req.gql.dart';
import 'package:typie/screens/site_settings/__generated__/update_site_slug_mutation.req.gql.dart';
import 'package:typie/services/blob.dart';
import 'package:typie/services/site.dart';
import 'package:typie/widgets/forms/form.dart';
import 'package:typie/widgets/forms/select.dart';
import 'package:typie/widgets/forms/text_field.dart';
import 'package:typie/widgets/settings_screen.dart';
import 'package:typie/widgets/tappable.dart';

const _unavailableSiteSlugs = ['admin', 'app', 'cname', 'dev', 'docs', 'help', 'template', 'www'];

String _usersiteHost() {
  final host = Env.usersiteHost.trim();
  return host.replaceFirst(RegExp(r'^\*\.'), '').replaceFirst(RegExp(r'^\.'), '');
}

@RoutePage()
class SiteSettingsScreen extends HookWidget {
  const SiteSettingsScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final site = useService<Site>();
    final siteId = useValueListenable(site);
    final scrollController = useScrollController();

    return GraphQLOperation(
      operation: GSiteSettingsScreen_QueryReq((b) => b..vars.siteId = siteId),
      builder: (context, client, data) {
        final sites = data.me?.sites.toList() ?? [];
        final isLastSite = sites.length <= 1;

        return SettingsOverlayScreen(
          title: '스페이스 설정',
          scrollController: scrollController,
          resizeToAvoidBottomInset: true,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            spacing: settingsSectionGap,
            children: [
              _GeneralTab(client: client, site: data.site, hasSubscription: data.me?.subscription != null),
              _DesignTab(client: client, site: data.site),
              _DangerZone(
                client: client,
                site: data.site,
                siteService: site,
                isLastSite: isLastSite,
                remainingSiteIds: sites.map((s) => s.id).where((id) => id != data.site.id).toList(),
              ),
            ],
          ),
        );
      },
    );
  }
}

class _GeneralTab extends HookWidget {
  const _GeneralTab({required this.client, required this.site, required this.hasSubscription});

  final GraphQLClient client;
  final GSiteSettingsScreen_QueryData_site site;
  final bool hasSubscription;

  @override
  Widget build(BuildContext context) {
    final blob = useService<Blob>();
    final mixpanel = useService<Mixpanel>();

    final logoUrl = useState(site.logo.url);
    final nameValue = useState(site.name);
    final slugValue = useState(site.slug);

    final nameForm = useHookForm();
    final slugForm = useHookForm();
    final nameController = useTextEditingController(text: nameValue.value);
    final slugController = useTextEditingController(text: slugValue.value);
    final nameFocusNode = useFocusNode();
    final slugFocusNode = useFocusNode();

    useEffect(() {
      void listener() {
        if (!nameFocusNode.hasFocus) {
          unawaited(nameForm.submit());
        }
      }

      nameFocusNode.addListener(listener);
      return () => nameFocusNode.removeListener(listener);
    }, [nameFocusNode, nameForm]);

    useEffect(() {
      void listener() {
        if (hasSubscription && !slugFocusNode.hasFocus) {
          unawaited(slugForm.submit());
        }
      }

      slugFocusNode.addListener(listener);
      return () => slugFocusNode.removeListener(listener);
    }, [hasSubscription, slugFocusNode, slugForm]);

    Future<void> updateSiteLogo() async {
      try {
        final result = await FilePicker.platform.pickFiles(type: FileType.image);
        if (result == null) {
          return;
        }

        if (result.files.isEmpty || result.files.first.path == null) {
          return;
        }

        final file = File(result.files.first.path!);
        final path = await blob.upload(file);
        final image = await client.request(
          GSiteSettingsScreen_PersistBlobAsImage_MutationReq(
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

        final updateResult = await client.request(
          GSiteSettingsScreen_UpdateSite_MutationReq(
            (b) => b
              ..vars.input.siteId = site.id
              ..vars.input.logoId = Value.present(image.persistBlobAsImage.id),
          ),
        );

        logoUrl.value = updateResult.updateSite.logo.url;
        unawaited(mixpanel.track('update_site_logo', properties: {'via': 'site_settings'}));

        if (context.mounted) {
          context.toast(ToastType.success, '스페이스 로고가 변경되었어요.');
        }
      } catch (_) {
        if (context.mounted) {
          context.toast(ToastType.error, '오류가 발생했어요. 잠시 후 다시 시도해주세요.');
        }
      }
    }

    const logoSize = 80.0;
    final imageSize = pow(2, (log(logoSize * MediaQuery.devicePixelRatioOf(context)) / log(2)).ceil()).toInt();
    final usersiteHost = _usersiteHost();

    return _Section(
      title: '일반',
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Padding(
            padding: const Pad(top: 24, bottom: 20),
            child: Center(
              child: Tappable(
                onTap: updateSiteLogo,
                child: Stack(
                  children: [
                    ClipRRect(
                      borderRadius: BorderRadius.circular(14),
                      child: CachedNetworkImage(
                        imageUrl: '${logoUrl.value}?s=$imageSize&q=75',
                        width: logoSize,
                        height: logoSize,
                        fit: BoxFit.cover,
                        fadeInDuration: Duration.zero,
                        fadeOutDuration: Duration.zero,
                        placeholderFadeInDuration: Duration.zero,
                      ),
                    ),
                    Container(
                      width: logoSize,
                      height: logoSize,
                      decoration: BoxDecoration(
                        borderRadius: BorderRadius.circular(14),
                        color: context.colors.textDefault.withValues(alpha: 0.15),
                      ),
                      child: Icon(LucideLightIcons.camera, size: 28, color: context.colors.textBright),
                    ),
                  ],
                ),
              ),
            ),
          ),
          HookForm(
            form: nameForm,
            schema: l.schema({
              'name': l.string().min(1, message: '스페이스 이름을 입력해주세요.').required(message: '스페이스 이름을 입력해주세요.'),
            }),
            onSubmit: (form) async {
              final rawName = (form.data['name'] as String?) ?? '';
              final nextName = rawName.trim();

              if (nextName == nameValue.value) {
                if (rawName != nextName) {
                  nameController.text = nextName;
                }
                return;
              }

              try {
                await client.request(
                  GSiteSettingsScreen_UpdateSite_MutationReq(
                    (b) => b
                      ..vars.input.siteId = site.id
                      ..vars.input.name = Value.present(nextName),
                  ),
                );

                nameValue.value = nextName;
                nameController.text = nextName;
                unawaited(mixpanel.track('update_site_name', properties: {'via': 'site_settings'}));

                if (context.mounted) {
                  context.toast(ToastType.success, '스페이스 이름이 변경되었어요.');
                }
              } catch (_) {
                if (context.mounted) {
                  context.toast(ToastType.error, '오류가 발생했어요. 잠시 후 다시 시도해주세요.');
                }
              }
            },
            builder: (context, form) {
              return Padding(
                padding: const Pad(all: 16),
                child: HookFormTextField(
                  name: 'name',
                  label: '이름',
                  placeholder: '스페이스 이름',
                  initialValue: site.name,
                  controller: nameController,
                  focusNode: nameFocusNode,
                  errorBelowField: true,
                  onChanged: (_) {
                    form.clearError('name');
                  },
                ),
              );
            },
          ),
          HookForm(
            form: slugForm,
            schema: l.schema({
              'slug': l
                  .string()
                  .min(4, message: '스페이스 주소는 4글자 이상이여야 해요')
                  .max(63, message: '스페이스 주소는 63글자를 넘을 수 없어요')
                  .regex(r'^[\da-z-]+$', message: '스페이스 주소는 소문자, 숫자, 하이픈만 사용할 수 있어요')
                  .regex(r'^(?!.*--)[\da-z-]+$', message: '하이픈을 연속으로 사용할 수 없어요')
                  .regex(r'^[\da-z][\da-z-]*[\da-z]$', message: '스페이스 주소는 하이픈으로 시작하거나 끝날 수 없어요')
                  .custom((value) => !_unavailableSiteSlugs.contains(value), message: '사용할 수 없는 스페이스 주소에요')
                  .required(message: '스페이스 주소를 입력해 주세요'),
            }),
            onSubmit: (form) async {
              if (!hasSubscription) {
                return;
              }

              final rawSlug = (form.data['slug'] as String?) ?? '';
              final nextSlug = rawSlug.trim().toLowerCase();
              if (nextSlug == slugValue.value) {
                if (rawSlug != nextSlug) {
                  slugController.text = nextSlug;
                }
                return;
              }

              try {
                await client.request(
                  GSiteSettingsScreen_UpdateSiteSlug_MutationReq(
                    (b) => b
                      ..vars.input.siteId = site.id
                      ..vars.input.slug = nextSlug,
                  ),
                );

                slugValue.value = nextSlug;
                slugController.text = nextSlug;
                unawaited(mixpanel.track('update_site_slug', properties: {'via': 'site_settings'}));

                if (context.mounted) {
                  context.toast(ToastType.success, '스페이스 주소가 변경되었어요.');
                }
              } on TypieError catch (e) {
                if (e.code == 'site_slug_already_exists') {
                  form.setError('slug', '이미 존재하는 스페이스 주소예요.');
                  return;
                }

                if (context.mounted) {
                  context.toast(ToastType.error, '오류가 발생했어요. 잠시 후 다시 시도해주세요.');
                }
              } catch (_) {
                if (context.mounted) {
                  context.toast(ToastType.error, '오류가 발생했어요. 잠시 후 다시 시도해주세요.');
                }
              }
            },
            builder: (context, form) {
              return Padding(
                padding: const Pad(all: 16),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  spacing: 8,
                  children: [
                    Stack(
                      children: [
                        AbsorbPointer(
                          absorbing: !hasSubscription,
                          child: HookFormTextField(
                            name: 'slug',
                            label: '주소',
                            placeholder: '스페이스 주소',
                            keyboardType: TextInputType.url,
                            initialValue: site.slug,
                            controller: slugController,
                            focusNode: slugFocusNode,
                            errorBelowField: true,
                            onChanged: (_) {
                              form.clearError('slug');
                            },
                            suffix: Text(
                              '.$usersiteHost',
                              style: TextStyle(
                                fontSize: 14,
                                fontWeight: FontWeight.w500,
                                color: context.colors.textSubtle,
                              ),
                            ),
                          ),
                        ),
                        if (!hasSubscription)
                          Positioned.fill(
                            child: Tappable(
                              onTap: () async {
                                await context.router.push(const EnrollPlanRoute());
                              },
                              child: const SizedBox.expand(),
                            ),
                          ),
                      ],
                    ),
                    if (!hasSubscription)
                      Text(
                        '스페이스 주소 기능은 FULL ACCESS 플랜에서 사용할 수 있어요.',
                        style: TextStyle(fontSize: 13, color: context.colors.textFaint),
                      ),
                  ],
                ),
              );
            },
          ),
        ],
      ),
    );
  }
}

class _DesignTab extends HookWidget {
  const _DesignTab({required this.client, required this.site});

  final GraphQLClient client;
  final GSiteSettingsScreen_QueryData_site site;

  @override
  Widget build(BuildContext context) {
    final mixpanel = useService<Mixpanel>();
    final currentDateDisplay = useState(site.dateDisplay);

    return _Section(
      title: '디자인',
      child: HookForm(
        submitMode: HookFormSubmitMode.onChange,
        onSubmit: (form) async {
          final nextDateDisplay = form.data['dateDisplay'] as GSiteDateDisplay;
          if (nextDateDisplay == currentDateDisplay.value) {
            return;
          }

          try {
            await client.request(
              GSiteSettingsScreen_UpdateSite_MutationReq(
                (b) => b
                  ..vars.input.siteId = site.id
                  ..vars.input.dateDisplay = Value.present(nextDateDisplay),
              ),
            );

            currentDateDisplay.value = nextDateDisplay;
            unawaited(
              mixpanel.track(
                'update_site_date_display',
                properties: {'value': nextDateDisplay.name, 'via': 'site_settings'},
              ),
            );

            if (context.mounted) {
              context.toast(ToastType.success, '날짜 표시 설정이 변경되었어요.');
            }
          } catch (_) {
            form.setValue('dateDisplay', currentDateDisplay.value, notify: false);
            if (context.mounted) {
              context.toast(ToastType.error, '오류가 발생했어요. 잠시 후 다시 시도해주세요.');
            }
          }
        },
        builder: (context, form) {
          return Padding(
            padding: const Pad(all: 16),
            child: Row(
              children: [
                const Expanded(
                  child: Text('글 목록에 표시할 날짜', style: TextStyle(fontSize: 16, fontWeight: FontWeight.w500)),
                ),
                HookFormSelect<GSiteDateDisplay>(
                  name: 'dateDisplay',
                  initialValue: currentDateDisplay.value,
                  items: const [
                    HookFormSelectItem(label: '최초 생성 시각', value: GSiteDateDisplay.CREATED_AT),
                    HookFormSelectItem(label: '마지막 수정 시각', value: GSiteDateDisplay.UPDATED_AT),
                    HookFormSelectItem(label: '미표시', value: GSiteDateDisplay.NONE),
                  ],
                ),
              ],
            ),
          );
        },
      ),
    );
  }
}

class _DangerZone extends HookWidget {
  const _DangerZone({
    required this.client,
    required this.site,
    required this.siteService,
    required this.isLastSite,
    required this.remainingSiteIds,
  });

  final GraphQLClient client;
  final GSiteSettingsScreen_QueryData_site site;
  final Site siteService;
  final bool isLastSite;
  final List<String> remainingSiteIds;

  @override
  Widget build(BuildContext context) {
    final mixpanel = useService<Mixpanel>();
    final totalCount = site.documentCount + site.folderCount;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        const SettingsSectionLabel(text: '위험 구역'),
        SettingsSectionCard(
          clipBehavior: Clip.antiAlias,
          child: Tappable(
            onTap: () async {
              if (isLastSite) {
                await context.showModal(
                  child: const AlertModal(
                    title: '스페이스를 삭제할 수 없어요',
                    message: '최소 1개의 스페이스가 필요해요.\n새 스페이스를 만든 후 삭제할 수 있어요.',
                  ),
                );
                return;
              }

              final deleted = await context.showBottomSheet<bool>(
                child: _DeleteSiteConfirmSheet(
                  client: client,
                  site: site,
                  siteService: siteService,
                  mixpanel: mixpanel,
                  totalCount: totalCount,
                  remainingSiteIds: remainingSiteIds,
                ),
              );

              if ((deleted ?? false) && context.mounted) {
                await context.router.maybePop();
              }
            },
            padding: const Pad(horizontal: 16),
            child: Tappable.scale(
              child: SizedBox(
                height: settingsListRowHeight,
                child: Row(
                  children: [
                    Expanded(
                      child: Text(
                        '스페이스 삭제',
                        style: TextStyle(fontSize: 16, fontWeight: FontWeight.w500, color: context.colors.textDanger),
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ),
        ),
      ],
    );
  }
}

class _DeleteSiteConfirmSheet extends HookWidget {
  const _DeleteSiteConfirmSheet({
    required this.client,
    required this.site,
    required this.siteService,
    required this.mixpanel,
    required this.totalCount,
    required this.remainingSiteIds,
  });

  final GraphQLClient client;
  final GSiteSettingsScreen_QueryData_site site;
  final Site siteService;
  final Mixpanel mixpanel;
  final int totalCount;
  final List<String> remainingSiteIds;

  @override
  Widget build(BuildContext context) {
    final controller = useTextEditingController();
    final inputValue = useValueListenable(controller);
    final confirmText = '$totalCount';
    final isConfirmed = totalCount == 0 || inputValue.text == confirmText;

    return ConfirmBottomSheet(
      title: '스페이스 삭제',
      message: '이 스페이스의 모든 글과 데이터가 삭제되며, 복구할 수 없어요.',
      confirmText: '삭제',
      shouldDismissOnConfirm: false,
      confirmTextColor: isConfirmed ? context.colors.textBright : context.colors.textFaint,
      confirmBackgroundColor: isConfirmed ? context.colors.accentDanger : context.colors.surfaceMuted,
      child: totalCount > 0
          ? Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              spacing: 8,
              children: [
                Text(
                  '삭제를 확인하려면 이 스페이스의 항목 수($totalCount)를 입력해주세요.',
                  style: TextStyle(fontSize: 14, color: context.colors.textSubtle),
                ),
                TextField(
                  controller: controller,
                  keyboardType: TextInputType.number,
                  decoration: InputDecoration(
                    hintText: confirmText,
                    border: const OutlineInputBorder(),
                    contentPadding: const Pad(horizontal: 12, vertical: 8),
                  ),
                ),
              ],
            )
          : null,
      onConfirm: () async {
        if (!isConfirmed) {
          return;
        }

        try {
          await client.request(GSiteSettingsScreen_DeleteSite_MutationReq((b) => b..vars.input.siteId = site.id));

          unawaited(mixpanel.track('delete_site'));

          siteService.setSiteId(remainingSiteIds.first);

          if (context.mounted) {
            context.toast(ToastType.success, '스페이스가 삭제되었어요.');
            context.router.pop(true);
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

class _Section extends StatelessWidget {
  const _Section({required this.title, required this.child});

  final String title;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SettingsSectionLabel(text: title),
        SettingsSectionCard(clipBehavior: Clip.antiAlias, child: child),
      ],
    );
  }
}
