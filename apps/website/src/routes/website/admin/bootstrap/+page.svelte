<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import dayjs from 'dayjs';
  import { graphql } from '$graphql';

  type Bootstrap = {
    version: number;
    updatedAt: string;
    maintenance: {
      enabled: boolean;
      title: string;
      message: string;
      until: string | null;
      platforms: ('ios' | 'android' | 'web')[];
      allowedIps: string[];
    };
    minVersion: {
      ios: { version: string; storeUrl: string };
      android: { version: string; storeUrl: string };
    };
  };

  const query = graphql(`
    query AdminBootstrap_Query {
      getBootstrap
    }
  `);

  const updateBootstrapMutation = graphql(`
    mutation AdminBootstrap_UpdateBootstrap_Mutation($input: UpdateBootstrapInput!) {
      updateBootstrap(input: $input)
    }
  `);

  let data = $state<Bootstrap | null>(null);
  let loading = $state(true);
  let saving = $state(false);
  let message = $state<{ type: 'success' | 'error'; text: string } | null>(null);

  $effect(() => {
    if ($query) {
      data = $query.getBootstrap as Bootstrap;
      loading = false;
    }
  });

  async function handleSubmit(e: SubmitEvent) {
    e.preventDefault();
    if (!data) return;

    saving = true;
    message = null;

    try {
      const { version, updatedAt, ...rest } = data;
      void version;
      void updatedAt;
      await updateBootstrapMutation({ bootstrap: rest });
      message = { type: 'success', text: 'BOOTSTRAP CONFIG UPDATED SUCCESSFULLY' };
    } catch (err) {
      message = { type: 'error', text: err instanceof Error ? err.message : 'FAILED TO UPDATE BOOTSTRAP CONFIG' };
    } finally {
      saving = false;
    }
  }

  let newIp = $state('');

  function addIp() {
    if (!data || !newIp.trim()) return;
    const ip = newIp.trim();
    if (!data.maintenance.allowedIps.includes(ip)) {
      data.maintenance.allowedIps = [...data.maintenance.allowedIps, ip];
    }
    newIp = '';
  }

  function removeIp(ip: string) {
    if (!data) return;
    data.maintenance.allowedIps = data.maintenance.allowedIps.filter((i) => i !== ip);
  }

  function togglePlatform(platform: 'ios' | 'android' | 'web') {
    if (!data) return;
    const platforms = data.maintenance.platforms;
    const index = platforms.indexOf(platform);
    if (index === -1) {
      data.maintenance.platforms = [...platforms, platform];
    } else {
      data.maintenance.platforms = platforms.filter((p) => p !== platform);
    }
  }

  const inputStyle = css({
    width: 'full',
    paddingX: '12px',
    paddingY: '8px',
    borderWidth: '2px',
    borderColor: 'amber.500',
    backgroundColor: 'gray.800',
    color: 'amber.500',
    fontSize: '13px',
    outline: 'none',
    caretColor: 'amber.500',
    _focus: {
      borderColor: 'amber.400',
    },
    _placeholder: {
      color: 'amber.700',
    },
  });

  const labelStyle = css({ fontSize: '11px', color: 'amber.400' });

  const sectionStyle = css({
    borderWidth: '2px',
    borderColor: 'amber.500',
    backgroundColor: 'gray.900',
  });

  const sectionHeaderStyle = css({
    padding: '16px',
    borderBottomWidth: '2px',
    borderColor: 'amber.500',
    fontSize: '14px',
    color: 'amber.500',
  });

  const sectionBodyStyle = css({ padding: '20px' });

  const textareaStyle = css({
    width: 'full',
    paddingX: '12px',
    paddingY: '8px',
    borderWidth: '2px',
    borderColor: 'amber.500',
    backgroundColor: 'gray.800',
    color: 'amber.500',
    fontSize: '13px',
    outline: 'none',
    caretColor: 'amber.500',
    minHeight: '80px',
    resize: 'vertical',
    _focus: {
      borderColor: 'amber.400',
    },
    _placeholder: {
      color: 'amber.700',
    },
  });
</script>

<div class={flex({ flexDirection: 'column', gap: '24px', color: 'amber.500' })}>
  <div>
    <h2 class={css({ fontSize: '18px', color: 'amber.500' })}>BOOTSTRAP CONFIG</h2>
    <p class={css({ marginTop: '8px', fontSize: '13px', color: 'amber.400' })}>SERVICE STATUS AND VERSION CONTROL</p>
  </div>

  {#if loading}
    <div class={css({ fontSize: '13px', color: 'amber.400' })}>LOADING...</div>
  {:else if data}
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
                    borderColor: 'amber.500',
                    backgroundColor: data.maintenance.enabled ? 'amber.500' : 'gray.800',
                    position: 'relative',
                    cursor: 'pointer',
                  })}
                  aria-label="Toggle maintenance mode"
                  onclick={() => {
                    if (data) data.maintenance.enabled = !data.maintenance.enabled;
                  }}
                  type="button"
                >
                  <div
                    style={`left: ${data.maintenance.enabled ? '26px' : '2px'}`}
                    class={css({
                      position: 'absolute',
                      top: '2px',
                      width: '16px',
                      height: '16px',
                      backgroundColor: data.maintenance.enabled ? 'gray.900' : 'amber.500',
                      transitionProperty: '[left]',
                      transitionDuration: '0.2s',
                    })}
                  ></div>
                </button>
                <span class={css({ fontSize: '11px', color: data.maintenance.enabled ? 'amber.500' : 'amber.700' })}>
                  {data.maintenance.enabled ? 'ON' : 'OFF'}
                </span>
              </div>

              <div class={flex({ flexDirection: 'column', gap: '8px' })}>
                <label class={labelStyle} for="maintenance-title">TITLE</label>
                <input
                  id="maintenance-title"
                  class={inputStyle}
                  placeholder="서비스 점검 중"
                  type="text"
                  bind:value={data.maintenance.title}
                />
              </div>

              <div class={flex({ flexDirection: 'column', gap: '8px' })}>
                <label class={labelStyle} for="maintenance-message">MESSAGE</label>
                <textarea
                  id="maintenance-message"
                  class={textareaStyle}
                  placeholder="점검 안내 메시지"
                  bind:value={data.maintenance.message}
                ></textarea>
              </div>

              <div class={flex({ flexDirection: 'column', gap: '8px' })}>
                <label class={labelStyle} for="maintenance-until">UNTIL (OPTIONAL)</label>
                <input
                  id="maintenance-until"
                  class={inputStyle}
                  oninput={(e) => {
                    if (!data) return;
                    const value = e.currentTarget.value;
                    data.maintenance.until = value ? new Date(value).toISOString() : null;
                  }}
                  type="datetime-local"
                  value={data.maintenance.until ? dayjs(data.maintenance.until).format('YYYY-MM-DDTHH:mm') : ''}
                />
              </div>

              <div class={flex({ flexDirection: 'column', gap: '8px' })}>
                <span class={labelStyle}>PLATFORMS</span>
                <div class={flex({ gap: '16px' })}>
                  {#each ['ios', 'android', 'web'] as platform (platform)}
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
                      onclick={() => togglePlatform(platform as 'ios' | 'android' | 'web')}
                      type="button"
                    >
                      <div
                        class={css({
                          width: '16px',
                          height: '16px',
                          borderWidth: '2px',
                          borderColor: 'amber.500',
                          backgroundColor: data.maintenance.platforms.includes(platform as 'ios' | 'android' | 'web')
                            ? 'amber.500'
                            : 'transparent',
                        })}
                      ></div>
                      <span class={css({ fontSize: '12px', color: 'amber.500' })}>{platform.toUpperCase()}</span>
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
                      if (e.key === 'Enter') {
                        e.preventDefault();
                        addIp();
                      }
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
                      borderColor: 'amber.500',
                      backgroundColor: 'amber.500',
                      color: 'gray.900',
                      fontSize: '13px',
                      cursor: 'pointer',
                      whiteSpace: 'nowrap',
                      _hover: {
                        backgroundColor: 'amber.400',
                        borderColor: 'amber.400',
                      },
                    })}
                    onclick={addIp}
                    type="button"
                  >
                    ADD
                  </button>
                </div>

                {#if data.maintenance.allowedIps.length === 0}
                  <div class={css({ fontSize: '12px', color: 'amber.700' })}>NO ALLOWED IPS</div>
                {:else}
                  <div class={flex({ flexDirection: 'column', gap: '4px' })}>
                    {#each data.maintenance.allowedIps as ip (ip)}
                      <div
                        class={flex({
                          alignItems: 'center',
                          justifyContent: 'space-between',
                          paddingX: '12px',
                          paddingY: '8px',
                          borderWidth: '1px',
                          borderColor: 'amber.700',
                        })}
                      >
                        <span class={css({ fontSize: '13px', color: 'amber.500' })}>{ip}</span>
                        <button
                          class={css({
                            backgroundColor: 'transparent',
                            border: 'none',
                            color: 'red.500',
                            fontSize: '12px',
                            cursor: 'pointer',
                            _hover: { color: 'red.400' },
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
              <div class={css({ borderWidth: '1px', borderColor: 'amber.700', padding: '16px' })}>
                <div class={css({ fontSize: '12px', color: 'amber.500', marginBottom: '12px' })}>iOS</div>
                <div class={flex({ gap: '16px' })}>
                  <div class={flex({ flexDirection: 'column', gap: '8px', flex: '1' })}>
                    <label class={labelStyle} for="ios-version">VERSION</label>
                    <input id="ios-version" class={inputStyle} placeholder="1.2.0" type="text" bind:value={data.minVersion.ios.version} />
                  </div>
                  <div class={css(flex.raw({ flexDirection: 'column', gap: '8px' }), { flexGrow: '2' })}>
                    <label class={labelStyle} for="ios-store-url">STORE URL</label>
                    <input
                      id="ios-store-url"
                      class={inputStyle}
                      placeholder="https://apps.apple.com/app/..."
                      type="url"
                      bind:value={data.minVersion.ios.storeUrl}
                    />
                  </div>
                </div>
              </div>

              <div class={css({ borderWidth: '1px', borderColor: 'amber.700', padding: '16px' })}>
                <div class={css({ fontSize: '12px', color: 'amber.500', marginBottom: '12px' })}>ANDROID</div>
                <div class={flex({ gap: '16px' })}>
                  <div class={flex({ flexDirection: 'column', gap: '8px', flex: '1' })}>
                    <label class={labelStyle} for="android-version">VERSION</label>
                    <input
                      id="android-version"
                      class={inputStyle}
                      placeholder="1.2.0"
                      type="text"
                      bind:value={data.minVersion.android.version}
                    />
                  </div>
                  <div class={css(flex.raw({ flexDirection: 'column', gap: '8px' }), { flexGrow: '2' })}>
                    <label class={labelStyle} for="android-store-url">STORE URL</label>
                    <input
                      id="android-store-url"
                      class={inputStyle}
                      placeholder="https://play.google.com/store/apps/..."
                      type="url"
                      bind:value={data.minVersion.android.storeUrl}
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
              borderColor: message.type === 'success' ? 'green.500' : 'red.500',
              color: message.type === 'success' ? 'green.500' : 'red.500',
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
              borderColor: 'amber.500',
              backgroundColor: 'amber.500',
              color: 'gray.900',
              fontSize: '13px',
              cursor: 'pointer',
              _hover: {
                backgroundColor: 'amber.400',
                borderColor: 'amber.400',
              },
              _disabled: {
                opacity: '50',
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
    <div class={css({ fontSize: '13px', color: 'red.500' })}>FAILED TO LOAD BOOTSTRAP CONFIG</div>
  {/if}
</div>
