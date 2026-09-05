<script lang="ts">
  import { createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import dayjs from 'dayjs';
  import { hydrateQuery } from '$lib/graphql';
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
  let message = $state<{ type: 'success' | 'error'; text: string } | null>(null);

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
    message = null;

    try {
      const { version, updatedAt, ...rest } = bootstrapData;
      void version;
      void updatedAt;
      await updateBootstrapMutation({ input: { bootstrap: rest } });
      message = { type: 'success', text: 'BOOTSTRAP CONFIG UPDATED SUCCESSFULLY' };
    } catch (err) {
      message = { type: 'error', text: err instanceof Error ? err.message : 'FAILED TO UPDATE BOOTSTRAP CONFIG' };
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

  const inputStyle = css({
    width: 'full',
    paddingX: '12px',
    paddingY: '8px',
    borderWidth: '2px',
    borderColor: 'border.default',
    backgroundColor: 'surface.inset',
    color: 'text.default',
    fontSize: '13px',
    outline: 'none',
    caretColor: 'text.default',
    _focus: {
      borderColor: 'accent.default',
    },
    _placeholder: {
      color: 'text.hint',
    },
  });

  const labelStyle = css({ fontSize: '11px', color: 'text.muted' });

  const sectionStyle = css({
    borderWidth: '2px',
    borderColor: 'border.default',
    backgroundColor: 'surface.default',
  });

  const sectionHeaderStyle = css({
    padding: '16px',
    borderBottomWidth: '2px',
    borderColor: 'border.default',
    fontSize: '14px',
    color: 'text.default',
  });

  const sectionBodyStyle = css({ padding: '20px' });

  const textareaStyle = css({
    width: 'full',
    paddingX: '12px',
    paddingY: '8px',
    borderWidth: '2px',
    borderColor: 'border.default',
    backgroundColor: 'surface.inset',
    color: 'text.default',
    fontSize: '13px',
    outline: 'none',
    caretColor: 'text.default',
    minHeight: '80px',
    resize: 'vertical',
    _focus: {
      borderColor: 'accent.default',
    },
    _placeholder: {
      color: 'text.hint',
    },
  });
</script>

<div class={flex({ flexDirection: 'column', gap: '24px', color: 'text.default' })}>
  <div>
    <h2 class={css({ fontSize: '18px', color: 'text.default' })}>BOOTSTRAP CONFIG</h2>
    <p class={css({ marginTop: '8px', fontSize: '13px', color: 'text.muted' })}>SERVICE STATUS AND VERSION CONTROL</p>
  </div>

  {#if loading}
    <div class={css({ fontSize: '13px', color: 'text.muted' })}>LOADING...</div>
  {:else if bootstrapData}
    <form onsubmit={handleSubmit}>
      <div class={flex({ flexDirection: 'column', gap: '24px' })}>
        <div class={sectionStyle}>
          <div class={sectionHeaderStyle}>MAINTENANCE</div>
          <div class={sectionBodyStyle}>
            <div class={flex({ flexDirection: 'column', gap: '16px' })}>
              <div class={flex({ alignItems: 'center', gap: '12px' })}>
                <label class={labelStyle} for="maintenance-enabled">ENABLED</label>
                <button
                  id="maintenance-enabled"
                  class={css({
                    width: '48px',
                    height: '24px',
                    borderWidth: '2px',
                    borderColor: 'border.default',
                    backgroundColor: bootstrapData.maintenance.enabled ? 'accent.default' : 'surface.inset',
                    position: 'relative',
                    cursor: 'pointer',
                  })}
                  aria-label="Toggle maintenance mode"
                  onclick={() => {
                    if (bootstrapData) bootstrapData.maintenance.enabled = !bootstrapData.maintenance.enabled;
                  }}
                  type="button"
                >
                  <div
                    style={`left: ${bootstrapData.maintenance.enabled ? '26px' : '2px'}`}
                    class={css({
                      position: 'absolute',
                      top: '2px',
                      width: '16px',
                      height: '16px',
                      backgroundColor: 'surface.default',
                      transitionProperty: '[left]',
                      transitionDuration: '0.2s',
                    })}
                  ></div>
                </button>
                <span class={css({ fontSize: '11px', color: bootstrapData.maintenance.enabled ? 'text.default' : 'text.hint' })}>
                  {bootstrapData.maintenance.enabled ? 'ON' : 'OFF'}
                </span>
              </div>

              <div class={flex({ flexDirection: 'column', gap: '8px' })}>
                <label class={labelStyle} for="maintenance-title">TITLE</label>
                <input
                  id="maintenance-title"
                  class={inputStyle}
                  placeholder="서비스 점검 중"
                  type="text"
                  bind:value={bootstrapData.maintenance.title}
                />
              </div>

              <div class={flex({ flexDirection: 'column', gap: '8px' })}>
                <label class={labelStyle} for="maintenance-message">MESSAGE</label>
                <textarea
                  id="maintenance-message"
                  class={textareaStyle}
                  placeholder="점검 안내 메시지"
                  bind:value={bootstrapData.maintenance.message}></textarea>
              </div>

              <div class={flex({ flexDirection: 'column', gap: '8px' })}>
                <label class={labelStyle} for="maintenance-until">UNTIL (OPTIONAL)</label>
                <input
                  id="maintenance-until"
                  class={inputStyle}
                  oninput={(e) => {
                    if (!bootstrapData) return;
                    const value = e.currentTarget.value;
                    bootstrapData.maintenance.until = value ? new Date(value).toISOString() : null;
                  }}
                  type="datetime-local"
                  value={bootstrapData.maintenance.until ? dayjs(bootstrapData.maintenance.until).format('YYYY-MM-DDTHH:mm') : ''}
                />
              </div>

              <div class={flex({ flexDirection: 'column', gap: '8px' })}>
                <span class={labelStyle}>PLATFORMS</span>
                <div class={flex({ gap: '16px' })}>
                  {#each ['ios', 'android', 'web', 'api'] as platform (platform)}
                    <button
                      class={css({
                        display: 'flex',
                        alignItems: 'center',
                        gap: '8px',
                        cursor: 'pointer',
                        backgroundColor: 'transparent',
                        border: 'none',
                        padding: '0',
                      })}
                      onclick={() => togglePlatform(platform as 'ios' | 'android' | 'web' | 'api')}
                      type="button"
                    >
                      <div
                        class={css({
                          width: '16px',
                          height: '16px',
                          borderWidth: '2px',
                          borderColor: 'border.default',
                          backgroundColor: bootstrapData.maintenance.platforms.includes(platform as 'ios' | 'android' | 'web' | 'api')
                            ? 'accent.default'
                            : 'transparent',
                        })}
                      ></div>
                      <span class={css({ fontSize: '12px', color: 'text.default' })}>{platform.toUpperCase()}</span>
                    </button>
                  {/each}
                </div>
              </div>

              <div class={flex({ flexDirection: 'column', gap: '8px' })}>
                <span class={labelStyle}>ALLOWED IPS</span>
                <div class={flex({ gap: '8px' })}>
                  <input
                    class={inputStyle}
                    onkeydown={(e) => {
                      if (e.key !== 'Enter') {
                        return;
                      }

                      e.preventDefault();
                      addIp();
                    }}
                    placeholder="0.0.0.0"
                    type="text"
                    bind:value={newIp}
                  />
                  <button
                    class={css({
                      paddingX: '16px',
                      paddingY: '8px',
                      borderWidth: '2px',
                      borderColor: 'accent.default',
                      backgroundColor: 'accent.default',
                      color: 'surface.default',
                      fontSize: '13px',
                      cursor: 'pointer',
                      whiteSpace: 'nowrap',
                      _hover: {
                        backgroundColor: '[color-mix(in oklch, token(colors.accent.default) 88%, black)]',
                        borderColor: '[color-mix(in oklch, token(colors.accent.default) 88%, black)]',
                      },
                    })}
                    onclick={addIp}
                    type="button"
                  >
                    ADD
                  </button>
                </div>

                {#if bootstrapData.maintenance.allowedIps.length === 0}
                  <div class={css({ fontSize: '12px', color: 'text.hint' })}>NO ALLOWED IPS</div>
                {:else}
                  <div class={flex({ flexDirection: 'column', gap: '4px' })}>
                    {#each bootstrapData.maintenance.allowedIps as ip (ip)}
                      <div
                        class={flex({
                          alignItems: 'center',
                          justifyContent: 'space-between',
                          paddingX: '12px',
                          paddingY: '8px',
                          borderWidth: '1px',
                          borderColor: 'border.default',
                        })}
                      >
                        <span class={css({ fontSize: '13px', color: 'text.default' })}>{ip}</span>
                        <button
                          class={css({
                            backgroundColor: 'transparent',
                            border: 'none',
                            paddingX: '12px',
                            paddingY: '6px',
                            color: 'danger.default',
                            fontSize: '12px',
                            cursor: 'pointer',
                            _hover: { backgroundColor: 'danger.default', color: 'text.on.danger' },
                          })}
                          onclick={() => removeIp(ip)}
                          type="button"
                        >
                          REMOVE
                        </button>
                      </div>
                    {/each}
                  </div>
                {/if}
              </div>
            </div>
          </div>
        </div>

        <div class={sectionStyle}>
          <div class={sectionHeaderStyle}>MIN VERSION</div>
          <div class={sectionBodyStyle}>
            <div class={flex({ flexDirection: 'column', gap: '16px' })}>
              <div class={css({ borderWidth: '1px', borderColor: 'border.default', padding: '16px' })}>
                <div class={css({ fontSize: '12px', color: 'text.muted', marginBottom: '12px' })}>iOS</div>
                <div class={flex({ gap: '16px' })}>
                  <div class={flex({ flexDirection: 'column', gap: '8px', flex: '1' })}>
                    <label class={labelStyle} for="ios-version">VERSION</label>
                    <input
                      id="ios-version"
                      class={inputStyle}
                      placeholder="1.2.0"
                      type="text"
                      bind:value={bootstrapData.minVersion.ios.version}
                    />
                  </div>
                  <div class={css(flex.raw({ flexDirection: 'column', gap: '8px' }), { flexGrow: '2' })}>
                    <label class={labelStyle} for="ios-store-url">STORE URL</label>
                    <input
                      id="ios-store-url"
                      class={inputStyle}
                      placeholder="https://apps.apple.com/app/..."
                      type="url"
                      bind:value={bootstrapData.minVersion.ios.storeUrl}
                    />
                  </div>
                </div>
              </div>

              <div class={css({ borderWidth: '1px', borderColor: 'border.default', padding: '16px' })}>
                <div class={css({ fontSize: '12px', color: 'text.muted', marginBottom: '12px' })}>ANDROID</div>
                <div class={flex({ gap: '16px' })}>
                  <div class={flex({ flexDirection: 'column', gap: '8px', flex: '1' })}>
                    <label class={labelStyle} for="android-version">VERSION</label>
                    <input
                      id="android-version"
                      class={inputStyle}
                      placeholder="1.2.0"
                      type="text"
                      bind:value={bootstrapData.minVersion.android.version}
                    />
                  </div>
                  <div class={css(flex.raw({ flexDirection: 'column', gap: '8px' }), { flexGrow: '2' })}>
                    <label class={labelStyle} for="android-store-url">STORE URL</label>
                    <input
                      id="android-store-url"
                      class={inputStyle}
                      placeholder="https://play.google.com/store/apps/..."
                      type="url"
                      bind:value={bootstrapData.minVersion.android.storeUrl}
                    />
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>

        {#if message}
          <div
            class={css({
              padding: '12px',
              borderWidth: '2px',
              borderColor: message.type === 'success' ? 'success.default' : 'danger.default',
              color: message.type === 'success' ? 'success.default' : 'danger.default',
              fontSize: '12px',
            })}
          >
            {message.text}
          </div>
        {/if}

        <div class={flex({ gap: '12px' })}>
          <button
            class={css({
              paddingX: '24px',
              paddingY: '12px',
              borderWidth: '2px',
              borderColor: 'accent.default',
              backgroundColor: 'accent.default',
              color: 'surface.default',
              fontSize: '13px',
              cursor: 'pointer',
              _hover: {
                backgroundColor: '[color-mix(in oklch, token(colors.accent.default) 88%, black)]',
                borderColor: '[color-mix(in oklch, token(colors.accent.default) 88%, black)]',
              },
              _disabled: {
                opacity: '40',
                cursor: 'not-allowed',
              },
            })}
            disabled={saving}
            type="submit"
          >
            {saving ? 'SAVING...' : 'SAVE CONFIG'}
          </button>
        </div>
      </div>
    </form>
  {:else}
    <div class={css({ fontSize: '13px', color: 'danger.default' })}>FAILED TO LOAD BOOTSTRAP CONFIG</div>
  {/if}
</div>
