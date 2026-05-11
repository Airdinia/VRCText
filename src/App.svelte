<script lang="ts">
  import { onMount } from "svelte";
  import Header from "./components/Header.svelte";
  import Composer from "./components/Composer.svelte";
  import HistoryList from "./components/HistoryList.svelte";
  import Settings from "./components/Settings.svelte";
  import Toast from "./components/Toast.svelte";
  import { initStore, store, toastBus } from "./lib/stores.svelte";

  let settingsOpen = $state(false);
  let toast = $state<{ text: string; tone: "info" | "error"; id: number } | null>(null);
  let toastId = 0;

  onMount(() => {
    let cleanup: (() => void) | null = null;
    void initStore().then((c) => {
      cleanup = c;
    });
    return () => cleanup?.();
  });

  function setStatus(text: string, tone: "info" | "error" = "info") {
    toast = { text, tone, id: ++toastId };
  }

  // Drain the cross-component toast bus through the same render path as
  // direct onStatus prop calls. Deep children (TtsSection inside Settings)
  // call pushToast() and land here.
  $effect(() => {
    const t = toastBus.value;
    if (t) {
      setStatus(t.text, t.tone);
      toastBus.value = null;
    }
  });
</script>

<main class="shell">
  <Header
    settingsOpen={settingsOpen}
    onToggleSettings={() => (settingsOpen = !settingsOpen)}
  />

  {#if store.loaded}
    <HistoryList onResendStatus={(t) => setStatus(t)} />
  {:else}
    <div class="loading">// loading</div>
  {/if}

  <Composer onStatus={setStatus} />

  {#if settingsOpen}
    <Settings onClose={() => (settingsOpen = false)} />
  {/if}

  {#if toast}
    {#key toast.id}
      <Toast text={toast.text} tone={toast.tone} />
    {/key}
  {/if}
</main>

<style>
  .shell {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg);
    overflow: hidden;
  }
  .loading {
    flex: 1;
    display: grid;
    place-items: center;
    color: var(--text-faint);
    font-size: 11px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
</style>
