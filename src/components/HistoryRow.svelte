<script lang="ts">
  import { RotateCcw } from "lucide-svelte";
  import { resendHistory } from "../lib/ipc";
  import type { HistoryEntry } from "../lib/ipc";
  import { requestAppend, setOscStatus } from "../lib/stores.svelte";

  interface Props {
    entry: HistoryEntry;
    index: number;
    onResend: (text: string) => void;
  }
  let { entry, index, onResend }: Props = $props();

  // Two-mode interaction on a history row (matches legacy egui app):
  //   • quick click (released within 500 ms, pointer moved < 5 px)
  //     → insert this text into the composer at the caret. Useful for
  //       riffing on a previous message before sending again.
  //   • long-press 500 ms with the pointer still on the row
  //     → straight resend through the OSC + TTS dispatch pipeline.
  // A move beyond SLOP_PX cancels both — that's the user starting a text
  // selection, which `user-select: text` on `.text` then makes work.
  const DURATION_MS = 500;
  const SLOP_PX = 5;

  let progress = $state(0);
  let pressing = $state(false);
  let hover = $state(false);
  let ripple = $state<{ x: number; y: number; k: number } | null>(null);

  let startedAt = 0;
  let rafId = 0;
  let startX = 0;
  let startY = 0;
  let longPressFired = false;

  function tick() {
    if (!pressing) return;
    const p = Math.min(1, (performance.now() - startedAt) / DURATION_MS);
    progress = p;
    if (p >= 1) {
      longPressFired = true;
      pressing = false;
      progress = 0;
      ripple = null;
      void doResend();
      return;
    }
    rafId = requestAnimationFrame(tick);
  }

  function onPointerDown(e: PointerEvent) {
    if (e.button !== 0) return;
    pressing = true;
    longPressFired = false;
    startedAt = performance.now();
    startX = e.clientX;
    startY = e.clientY;
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    ripple = {
      x: e.clientX - rect.left,
      y: e.clientY - rect.top,
      k: Math.random(),
    };
    rafId = requestAnimationFrame(tick);
  }

  function onPointerMove(e: PointerEvent) {
    if (!pressing) return;
    if (Math.hypot(e.clientX - startX, e.clientY - startY) > SLOP_PX) {
      cancel();
    }
  }

  function onPointerUp() {
    // Pointer released before the 500 ms long-press timer fired → treat as
    // a short click, splice into the composer.
    if (pressing && !longPressFired) {
      pressing = false;
      progress = 0;
      ripple = null;
      cancelAnimationFrame(rafId);
      requestAppend(entry.text);
      return;
    }
    cancel();
  }

  function cancel() {
    if (pressing) {
      pressing = false;
      progress = 0;
      ripple = null;
    }
    cancelAnimationFrame(rafId);
  }

  async function doResend() {
    try {
      await resendHistory(index);
      // See Composer: oscStatus is driven by the vrc-alive backend probe,
      // not by send_to() returning Ok. UDP can't tell us if VRChat got it.
      const preview =
        entry.text.length > 12
          ? `${entry.text.slice(0, 12)}…`
          : entry.text;
      onResend(`已重发 「${preview}」`);
    } catch (err) {
      setOscStatus("error");
      onResend(`重发失败: ${err}`);
    }
  }

  // Same-day → HH:mm, otherwise MM-DD HH:mm.
  const stamp = $derived.by(() => {
    const d = new Date(entry.ts * 1000);
    const now = new Date();
    const hh = `${d.getHours()}`.padStart(2, "0");
    const mm = `${d.getMinutes()}`.padStart(2, "0");
    if (
      d.getFullYear() === now.getFullYear() &&
      d.getMonth() === now.getMonth() &&
      d.getDate() === now.getDate()
    ) {
      return `${hh}:${mm}`;
    }
    const M = `${d.getMonth() + 1}`.padStart(2, "0");
    const D = `${d.getDate()}`.padStart(2, "0");
    return `${M}-${D} ${hh}:${mm}`;
  });
</script>

<div
  class="row"
  class:pressing
  role="button"
  tabindex="0"
  title="短按:追加到输入框｜长按 0.5s:直接重发"
  onmouseenter={() => (hover = true)}
  onmouseleave={() => {
    hover = false;
    cancel();
  }}
  onpointerdown={onPointerDown}
  onpointermove={onPointerMove}
  onpointerup={onPointerUp}
  onpointercancel={cancel}
>
  {#if ripple && pressing}
    {#key ripple.k}
      <span
        class="ripple"
        style:--x="{ripple.x}px"
        style:--y="{ripple.y}px"
        style:--scale={1 + progress * 50}
        style:--opacity={1 - progress * 0.6}
      ></span>
    {/key}
  {/if}

  <div class="line">
    <div class="text">
      <span class="prompt">{">"}</span>{entry.text}
    </div>
    <div class="side">
      <span class="ts">{stamp}</span>
      <span class="icon-wrap" class:hover>
        <RotateCcw />
      </span>
    </div>
  </div>

  {#if pressing}
    <div class="linear" aria-hidden="true">
      <div class="linear-fill" style:width="{(progress * 100).toFixed(1)}%"></div>
    </div>
  {/if}
</div>

<style>
  .row {
    position: relative;
    flex-shrink: 0;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    padding: 10px 12px;
    cursor: default;
    user-select: none;
    overflow: hidden;
    transition: background 120ms, transform 120ms, box-shadow 120ms,
      border-color 120ms;
  }
  .row:hover {
    background: var(--card-hover);
    transform: translateY(-1px);
    box-shadow: var(--shadow-md);
  }
  .row.pressing {
    background: var(--card-active);
    border-color: var(--border-strong);
    transform: translateY(0);
    box-shadow: none;
  }

  .line {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .text {
    flex: 1;
    min-width: 0;
    color: var(--text);
    font-size: 13.5px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .prompt {
    color: var(--accent);
    opacity: 0.7;
    margin-right: 6px;
  }
  .side {
    display: flex;
    align-items: center;
    color: var(--text-faint);
    font-size: 11px;
    font-variant-numeric: tabular-nums;
    flex-shrink: 0;
  }
  .ts {
    color: var(--text-faint);
  }
  .icon-wrap {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 0;
    height: 14px;
    margin-left: 0;
    opacity: 0;
    overflow: hidden;
    color: var(--text-faint);
    transition: width 150ms ease, opacity 150ms ease, margin-left 150ms ease,
      color 120ms;
  }
  .row:hover .icon-wrap {
    width: 14px;
    margin-left: 8px;
    opacity: 1;
  }
  .icon-wrap.hover {
    color: var(--accent);
  }
  .icon-wrap :global(svg) {
    width: 11px;
    height: 11px;
    stroke-width: 1.5;
  }

  /* Radial ripple expands from press point */
  .ripple {
    position: absolute;
    left: var(--x);
    top: var(--y);
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: radial-gradient(
      circle,
      var(--accent) 0%,
      var(--accent-soft) 35%,
      transparent 70%
    );
    transform: translate(-50%, -50%) scale(var(--scale));
    opacity: var(--opacity);
    pointer-events: none;
    mix-blend-mode: screen;
  }

  /* Linear progress at bottom edge */
  .linear {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 2px;
    background: var(--accent-soft);
  }
  .linear-fill {
    height: 100%;
    background: var(--accent);
  }
</style>
