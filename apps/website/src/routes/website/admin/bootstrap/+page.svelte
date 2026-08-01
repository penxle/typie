<script lang="ts">
  import { createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Icon, Switch, TextInput } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import dayjs from 'dayjs';
  import XIcon from '~icons/lucide/x';
  import { AdminEmpty, adminFilledControl, AdminPageHeader } from '$lib/components/admin';
  import { hydrateQuery, unwrapError } from '$lib/graphql';
  import { graphql } from '$mearie';

  let { data } = $props();

  const query = $derived(hydrateQuery(() => data.query));

  type Bootstrap = {
    version: number;
    updatedAt: string;
    maintenance: {
      enabled: boolean;
      title: string;
      message: string;
      until: string | null;
      platforms: ('ios' | 'android' | 'web' | 'api')[];
      allowedIps: string[];
    };
    minVersion: {
      ios: { version: string; storeUrl: string };
      android: { version: string; storeUrl: string };
    };
  };

  const [updateBootstrapMutation] = createMutation(
    graphql(`
      mutation AdminBootstrap_UpdateBootstrap_Mutation($input: UpdateBootstrapInput!) {
        updateBootstrap(input: $input)
      }
    `),
  );

  let bootstrapData = $state<Bootstrap | null>(null);
  let loading = $state(true);
  let saving = $state(false);

  $effect(() => {
    if (!query.data) {
      return;
    }

    bootstrapData = query.data.getBootstrap as Bootstrap;
    loading = false;
  });

  async function handleSubmit(e: SubmitEvent) {
    e.preventDefault();
    if (!bootstrapData) return;

    saving = true;

    try {
      const { version, updatedAt, ...rest } = bootstrapData;
      void version;
      void updatedAt;
      await updateBootstrapMutation({ input: { bootstrap: rest } });
      Toast.success('부트스트랩 설정을 저장했어요');
    } catch (err) {
      const unwrapped = unwrapError(err);
      Toast.error(unwrapped instanceof Error ? unwrapped.message : '저장에 실패했어요. 잠시 후 다시 시도해주세요.');
    } finally {
      saving = false;
    }
  }

  let newIp = $state('');

  function addIp() {
    if (!bootstrapData || !newIp.trim()) return;
    const ip = newIp.trim();
    if (!bootstrapData.maintenance.allowedIps.includes(ip)) {
      bootstrapData.maintenance.allowedIps = [...bootstrapData.maintenance.allowedIps, ip];
    }
    newIp = '';
  }

  function removeIp(ip: string) {
    if (!bootstrapData) return;
    bootstrapData.maintenance.allowedIps = bootstrapData.maintenance.allowedIps.filter((i) => i !== ip);
  }

  function togglePlatform(platform: 'ios' | 'android' | 'web' | 'api') {
    if (!bootstrapData) return;
    const platforms = bootstrapData.maintenance.platforms;
    const index = platforms.indexOf(platform);
    if (index === -1) {
      bootstrapData.maintenance.platforms = [...platforms, platform];
    } else {
      bootstrapData.maintenance.platforms = platforms.filter((p) => p !== platform);
    }
  }

  const platformLabels: Record<'ios' | 'android' | 'web' | 'api', string> = {
    ios: 'iOS',
    android: 'Android',
    web: 'Web',
    api: 'API',
  };

  const sectionStyle = css({
    borderWidth: '1px',
    borderColor: 'border.subtle',
    borderRadius: '12px',
    backgroundColor: 'admin.card.default',
    boxShadow: 'adminCard',
  });
  const sectionHeaderStyle = css({
    paddingX: '20px',
    paddingY: '14px',
    borderBottomWidth: '1px',
    borderColor: 'border.subtle',
    fontSize: '14px',
    fontWeight: 'semibold',
    color: 'text.default',
  });
  const sectionBodyStyle = css({ padding: '20px' });
  const fieldLabelStyle = css({ fontSize: '12px', fontWeight: 'medium', color: 'text.muted' });
  const subCardStyle = css({ borderRadius: '8px', padding: '16px', backgroundColor: 'surface.muted' });

  const textareaStyle = css({
    width: 'full',
    borderWidth: '1px',
    borderColor: 'transparent',
    borderRadius: '8px',
    backgroundColor: 'surface.muted',
    paddingX: '12px',
    paddingY: '8px',
    fontSize: '13px',
    color: 'text.default',
    outline: 'none',
    minHeight: '80px',
    resize: 'vertical',
    _focus: { borderColor: 'border.brand' },
    _placeholder: { color: 'text.disabled' },
  });
</script>

<AdminPageHeader description="점검 모드와 최소 지원 버전을 관리해요" title="부트스트랩 설정" />

{#if loading}
  <AdminEmpty text="불러오는 중이에요..." />
{:else if bootstrapData}
  <form onsubmit={handleSubmit}>
    <div class={flex({ flexDirection: 'column', gap: '20px' })}>
      <div class={sectionStyle}>
        <div class={sectionHeaderStyle}>점검 모드</div>
        <div class={sectionBodyStyle}>
          <div class={flex({ flexDirection: 'column', gap: '20px' })}>
            <div class={flex({ alignItems: 'center', gap: '8px' })}>
              <Switch bind:checked={bootstrapData.maintenance.enabled} />
              <span class={css({ fontSize: '13px', color: 'text.subtle' })}>
                {bootstrapData.maintenance.enabled ? '점검 모드가 켜져 있어요' : '점검 모드가 꺼져 있어요'}
              </span>
            </div>

            <div class={flex({ flexDirection: 'column', gap: '6px' })}>
              <span class={fieldLabelStyle}>제목</span>
              <TextInput
                style={css.raw(adminFilledControl)}
                placeholder="서비스 점검 중"
                size="sm"
                bind:value={bootstrapData.maintenance.title}
              />
            </div>

            <div class={flex({ flexDirection: 'column', gap: '6px' })}>
              <span class={fieldLabelStyle}>안내 메시지</span>
              <textarea class={textareaStyle} placeholder="점검 안내 메시지" bind:value={bootstrapData.maintenance.message}></textarea>
            </div>

            <div class={flex({ flexDirection: 'column', gap: '6px' })}>
              <span class={fieldLabelStyle}>점검 종료 시각(선택)</span>
              <TextInput
                style={css.raw(adminFilledControl)}
                oninput={(e) => {
                  if (!bootstrapData) return;
                  const value = e.currentTarget.value;
                  bootstrapData.maintenance.until = value ? new Date(value).toISOString() : null;
                }}
                size="sm"
                type="datetime-local"
                value={bootstrapData.maintenance.until ? dayjs(bootstrapData.maintenance.until).format('YYYY-MM-DDTHH:mm') : ''}
              />
            </div>

            <div class={flex({ flexDirection: 'column', gap: '8px' })}>
              <span class={fieldLabelStyle}>적용 플랫폼</span>
              <div class={flex({ gap: '20px' })}>
                {#each ['ios', 'android', 'web', 'api'] as platform (platform)}
                  {@const p = platform as 'ios' | 'android' | 'web' | 'api'}
                  <div class={flex({ alignItems: 'center', gap: '8px' })}>
                    <Switch checked={bootstrapData.maintenance.platforms.includes(p)} onclick={() => togglePlatform(p)} />
                    <span class={css({ fontSize: '13px', color: 'text.subtle' })}>{platformLabels[p]}</span>
                  </div>
                {/each}
              </div>
            </div>

            <div class={flex({ flexDirection: 'column', gap: '8px' })}>
              <span class={fieldLabelStyle}>허용 IP</span>
              <div class={flex({ gap: '8px' })}>
                <TextInput
                  style={css.raw(adminFilledControl, { flexGrow: '1' })}
                  onkeydown={(e) => {
                    if (e.key !== 'Enter') {
                      return;
                    }

                    e.preventDefault();
                    addIp();
                  }}
                  placeholder="0.0.0.0"
                  size="sm"
                  bind:value={newIp}
                />
                <Button onclick={addIp} size="sm" type="button" variant="secondary">추가</Button>
              </div>

              {#if bootstrapData.maintenance.allowedIps.length === 0}
                <div class={css({ fontSize: '12px', color: 'text.disabled' })}>등록된 IP가 없어요</div>
              {:else}
                <div class={flex({ flexDirection: 'column', gap: '6px' })}>
                  {#each bootstrapData.maintenance.allowedIps as ip (ip)}
                    <div
                      class={flex({
                        alignItems: 'center',
                        justifyContent: 'space-between',
                        borderRadius: '8px',
                        backgroundColor: 'surface.muted',
                        paddingX: '12px',
                        paddingY: '6px',
                      })}
                    >
                      <span class={css({ fontSize: '13px', color: 'text.subtle' })}>{ip}</span>
                      <Button onclick={() => removeIp(ip)} size="sm" type="button" variant="ghost">
                        <Icon icon={XIcon} size={14} />
                      </Button>
                    </div>
                  {/each}
                </div>
              {/if}
            </div>
          </div>
        </div>
      </div>

      <div class={sectionStyle}>
        <div class={sectionHeaderStyle}>최소 지원 버전</div>
        <div class={sectionBodyStyle}>
          <div class={flex({ flexDirection: 'column', gap: '16px' })}>
            <div class={subCardStyle}>
              <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.default', marginBottom: '12px' })}>iOS</div>
              <div class={flex({ gap: '16px' })}>
                <div class={flex({ flexDirection: 'column', gap: '6px', flex: '1' })}>
                  <span class={fieldLabelStyle}>버전</span>
                  <TextInput
                    style={css.raw(adminFilledControl)}
                    placeholder="1.2.0"
                    size="sm"
                    bind:value={bootstrapData.minVersion.ios.version}
                  />
                </div>
                <div class={css(flex.raw({ flexDirection: 'column', gap: '6px' }), { flexGrow: '2' })}>
                  <span class={fieldLabelStyle}>스토어 URL</span>
                  <TextInput
                    style={css.raw(adminFilledControl)}
                    placeholder="https://apps.apple.com/app/..."
                    size="sm"
                    type="url"
                    bind:value={bootstrapData.minVersion.ios.storeUrl}
                  />
                </div>
              </div>
            </div>

            <div class={subCardStyle}>
              <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.default', marginBottom: '12px' })}>Android</div>
              <div class={flex({ gap: '16px' })}>
                <div class={flex({ flexDirection: 'column', gap: '6px', flex: '1' })}>
                  <span class={fieldLabelStyle}>버전</span>
                  <TextInput
                    style={css.raw(adminFilledControl)}
                    placeholder="1.2.0"
                    size="sm"
                    bind:value={bootstrapData.minVersion.android.version}
                  />
                </div>
                <div class={css(flex.raw({ flexDirection: 'column', gap: '6px' }), { flexGrow: '2' })}>
                  <span class={fieldLabelStyle}>스토어 URL</span>
                  <TextInput
                    style={css.raw(adminFilledControl)}
                    placeholder="https://play.google.com/store/apps/..."
                    size="sm"
                    type="url"
                    bind:value={bootstrapData.minVersion.android.storeUrl}
                  />
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <div class={flex({ gap: '12px' })}>
        <Button disabled={saving} loading={saving} size="sm" type="submit">저장</Button>
      </div>
    </div>
  </form>
{:else}
  <AdminEmpty text="부트스트랩 설정을 불러오지 못했어요" />
{/if}
