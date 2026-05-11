<script lang="ts">
  import { Send, Loader2 } from "lucide-svelte";
  import { sendMessage, setTyping } from "../lib/ipc";
  import { MAX_CHARS, store, appendBus, setOscStatus } from "../lib/stores.svelte";
  import { t, tf, maybeT } from "../lib/i18n.svelte";

  interface Props {
    onStatus: (text: string, tone?: "info" | "error") => void;
  }
  let { onStatus }: Props = $props();

  let text = $state("");
  let sending = $state(false);
  let focused = $state(false);
  let textarea: HTMLTextAreaElement | null = $state(null);

  // History navigation cursor — null means "at the end (current draft)".
  let historyCursor: number | null = $state(null);
  let draftBeforeNav = "";

  // Typing-indicator scheduling — fully event-driven so the WebView
  // process can fall idle between keystrokes (the previous setInterval
  // fired 4×/s forever, even with the textarea empty).
  let typingActive = false;
  let idleOffTimer: ReturnType<typeof setTimeout> | null = null;
  let heartbeatTimer: ReturnType<typeof setTimeout> | null = null;
  const TYPING_HEARTBEAT_MS = 1500;
  const TYPING_IDLE_OFF_MS = 2000;

  $effect(() => {
    return stopTyping;
  });

  // Pick up "append this text" requests from HistoryRow's short-click and
  // splice them in at the textarea caret (or selection range). Mirrors the
  // legacy `append_to_input` flow that used egui's TextEdit cursor state.
  $effect(() => {
    const intent = appendBus.value;
    if (!intent || !textarea) return;
    const start = textarea.selectionStart ?? text.length;
    const end = textarea.selectionEnd ?? text.length;
    text = text.slice(0, start) + intent.text + text.slice(end);
    historyCursor = null;
    appendBus.value = null;
    // Re-set selection after Svelte flushes the new value back to the DOM.
    const caret = start + intent.text.length;
    queueMicrotask(() => {
      textarea?.focus();
      textarea?.setSelectionRange(caret, caret);
      // Re-grow textarea to fit any new line breaks from the inserted text.
      if (textarea) {
        textarea.style.height = "auto";
        textarea.style.height = Math.min(72, textarea.scrollHeight) + "px";
      }
    });
    noteKeystroke();
  });

  function noteKeystroke() {
    if (text.trim().length === 0) {
      stopTyping();
      return;
    }
    if (!typingActive) {
      typingActive = true;
      void setTyping(true);
      armHeartbeat();
    }
    armIdleOff();
  }

  function armIdleOff() {
    if (idleOffTimer !== null) clearTimeout(idleOffTimer);
    idleOffTimer = setTimeout(stopTyping, TYPING_IDLE_OFF_MS);
  }

  // Chained setTimeout (not setInterval) so the timer chain breaks
  // cleanly when stopTyping clears it — no zombie interval ticks.
  function armHeartbeat() {
    if (heartbeatTimer !== null) clearTimeout(heartbeatTimer);
    heartbeatTimer = setTimeout(() => {
      if (!typingActive) return;
      void setTyping(true);
      armHeartbeat();
    }, TYPING_HEARTBEAT_MS);
  }

  function stopTyping() {
    if (idleOffTimer !== null) {
      clearTimeout(idleOffTimer);
      idleOffTimer = null;
    }
    if (heartbeatTimer !== null) {
      clearTimeout(heartbeatTimer);
      heartbeatTimer = null;
    }
    if (typingActive) {
      typingActive = false;
      void setTyping(false);
    }
  }

  // Counter colors: default → warn (<15 left) → danger (over).
  const remaining = $derived(MAX_CHARS - [...text].length);
  const countTone = $derived(
    remaining < 0 ? "over" : remaining < 15 ? "warn" : "ok",
  );
  const countLabel = $derived(
    remaining < 0 ? tf("overflowCount", { n: -remaining }) : `${remaining}`,
  );

  async function doSend() {
    if (sending) return;
    const trimmed = text.trim();
    if (!trimmed) return;
    if (remaining < 0) {
      onStatus(tf("overflowToast", { n: MAX_CHARS }), "error");
      return;
    }
    sending = true;
    try {
      await sendMessage(trimmed);
      // oscStatus is NOT set to "ok" here — UDP send_to() always returns
      // Ok regardless of whether VRChat is actually listening. The dot
      // turns green only when the backend `vrc-alive` probe catches a
      // packet coming back from VRChat (see stores.svelte.ts).
      text = "";
      historyCursor = null;
      onStatus(t("sentToast"));
      if (textarea) textarea.style.height = "auto";
    } catch (err) {
      setOscStatus("error");
      onStatus(maybeT(String(err)), "error");
    } finally {
      sending = false;
      stopTyping();
      textarea?.focus();
    }
  }

  function onKeyDown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey && !e.isComposing) {
      e.preventDefault();
      void doSend();
      return;
    }

    if (e.key === "ArrowUp" && text.length === 0) {
      const hist = store.history;
      if (hist.length === 0) return;
      e.preventDefault();
      if (historyCursor === null) {
        draftBeforeNav = text;
        historyCursor = hist.length - 1;
      } else if (historyCursor > 0) {
        historyCursor -= 1;
      }
      text = hist[historyCursor].text;
      return;
    }

    if (e.key === "ArrowDown" && historyCursor !== null) {
      const hist = store.history;
      e.preventDefault();
      if (historyCursor < hist.length - 1) {
        historyCursor += 1;
        text = hist[historyCursor].text;
      } else {
        historyCursor = null;
        text = draftBeforeNav;
      }
      return;
    }
  }

  function onInput(e: Event) {
    const el = e.currentTarget as HTMLTextAreaElement;
    // Auto-grow up to 72px (≈3 lines), then internal scroll.
    el.style.height = "auto";
    el.style.height = Math.min(72, el.scrollHeight) + "px";
    if (historyCursor !== null) historyCursor = null;
    noteKeystroke();
  }
</script>

<section class="composer" class:focused>
  <!-- svelte-ignore a11y_autofocus -->
  <textarea
    bind:this={textarea}
    bind:value={text}
    placeholder={t("composerPlaceholder")}
    rows="1"
    onkeydown={onKeyDown}
    oninput={onInput}
    onfocus={() => (focused = true)}
    onblur={() => (focused = false)}
    autofocus
  ></textarea>
  <div class="meta">
    <div class="hint">
      <kbd>↵</kbd>
      <span class="hint-label" class:active={focused}>{t("sendHint")}</span>
    </div>
    <div class="right">
      <span class="counter" data-tone={countTone}>{countLabel}</span>
      <button
        class="send"
        type="button"
        disabled={sending || !text.trim() || remaining < 0}
        onclick={doSend}
        title={t("sendButtonTitle")}
        aria-label={t("sendButton")}
      >
        {#if sending}
          <Loader2 class="spin" />
        {:else}
          <Send />
        {/if}
      </button>
    </div>
  </div>
</section>

<style>
  .composer {
    flex-shrink: 0;
    margin: 6px 10px 10px;
    background: var(--card);
    border: 1px solid var(--border);
    /* Green left bar — the cursor accent for terminal feel */
    border-left: 2px solid var(--border-strong);
    border-radius: var(--r-md);
    transition: border-color 150ms, box-shadow 150ms;
    display: flex;
    flex-direction: column;
  }
  .composer.focused {
    border-color: var(--accent-ring);
    border-left-color: var(--accent);
  }
  textarea {
    background: transparent;
    border: 0;
    outline: 0;
    resize: none;
    min-height: 32px;
    width: 100%;
    padding: 10px 12px 6px;
    color: var(--text);
    font-family: inherit;
    font-size: 13.5px;
    line-height: 1.4;
    box-sizing: border-box;
  }
  textarea::placeholder {
    color: var(--text-faint);
  }
  .meta {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 8px 6px;
    gap: 6px;
  }
  .hint {
    display: inline-flex;
    align-items: center;
    color: var(--text-faint);
    font-size: 10.5px;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    padding-left: 4px;
  }
  kbd {
    display: inline-flex;
    align-items: center;
    padding: 1px 4px;
    border-radius: 3px;
    background: var(--surface);
    border: 1px solid var(--border);
    color: var(--text-muted);
    font-size: 9.5px;
    font-family: inherit;
    letter-spacing: 0;
  }
  .hint-label {
    margin-left: 6px;
    opacity: 0.6;
    transition: opacity 120ms;
  }
  .hint-label.active {
    opacity: 1;
  }
  .right {
    display: inline-flex;
    align-items: center;
    gap: 10px;
  }
  .counter {
    font-size: 11px;
    font-variant-numeric: tabular-nums;
    color: var(--text-faint);
  }
  .counter[data-tone="warn"] {
    color: var(--warn);
  }
  .counter[data-tone="over"] {
    color: var(--danger);
    font-weight: 600;
  }
  .send {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 24px;
    border-radius: var(--r-sm);
    background: var(--accent);
    color: var(--accent-text);
    box-shadow: 0 2px 8px -2px var(--accent-ring);
    transition: opacity 120ms, transform 80ms;
  }
  .send:active:not(:disabled) {
    transform: scale(0.95);
    opacity: 0.85;
  }
  .send:disabled {
    background: var(--surface);
    color: var(--text-faint);
    cursor: not-allowed;
    box-shadow: none;
  }
  .send :global(svg) {
    width: 12px;
    height: 12px;
    stroke-width: 1.5;
  }
  .send :global(.spin) {
    animation: vrc-spin 700ms linear infinite;
  }
</style>
