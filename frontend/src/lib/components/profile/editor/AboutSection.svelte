<script lang="ts">
  import { onMount } from 'svelte';
  import { Editor } from '@tiptap/core';
  import StarterKit from '@tiptap/starter-kit';
  import Link from '@tiptap/extension-link';
  import Underline from '@tiptap/extension-underline';
  import PromptDialog from '$lib/components/PromptDialog.svelte';

  let {
    initial = '',
    onSave
  }: {
    initial?: string;
    onSave: (html: string) => Promise<void> | void;
  } = $props();

  let el: HTMLDivElement | null = $state(null);
  let editor: Editor | null = null;
  let linkPromptOpen = $state(false);
  let dirty = $state(false);
  let saving = $state(false);

  onMount(() => {
    if (!el) return;
    editor = new Editor({
      element: el,
      extensions: [
        StarterKit.configure({
          // Allowlist: p, br, strong, em, u, h2, h3, h4, ul, ol, li, blockquote, code, a.
          // Disable starter-kit nodes/marks that emit non-allowed HTML.
          codeBlock: false,
          horizontalRule: false,
          heading: { levels: [2, 3, 4] }
        }),
        Underline,
        Link.configure({
          openOnClick: false,
          HTMLAttributes: { rel: 'nofollow noopener', target: '_blank' },
          protocols: ['http', 'https', 'mailto']
        })
      ],
      content: initial,
      editorProps: {
        attributes: {
          class: 'tiptap-bio'
        }
      },
      onUpdate: () => {
        dirty = true;
      },
      onBlur: () => {
        void handleSave();
      }
    });
    return () => {
      editor?.destroy();
      editor = null;
    };
  });

  async function handleSave() {
    if (!editor || !dirty || saving) return;
    saving = true;
    try {
      const html = editor.getHTML();
      await onSave(html);
      dirty = false;
    } finally {
      saving = false;
    }
  }

  function toggle(cmd: () => void) {
    return () => {
      cmd();
      el?.querySelector<HTMLElement>('.tiptap-bio')?.focus();
    };
  }
</script>

<section class="about-editor">
  <div class="toolbar" role="toolbar" aria-label="Bio formatting">
    <button type="button" onclick={toggle(() => editor?.chain().focus().toggleBold().run())}
      >B</button
    >
    <button type="button" onclick={toggle(() => editor?.chain().focus().toggleItalic().run())}
      ><em>I</em></button
    >
    <button type="button" onclick={toggle(() => editor?.chain().focus().toggleUnderline().run())}
      ><u>U</u></button
    >
    <button
      type="button"
      onclick={toggle(() => editor?.chain().focus().toggleHeading({ level: 2 }).run())}>H₂</button
    >
    <button
      type="button"
      onclick={toggle(() => editor?.chain().focus().toggleHeading({ level: 3 }).run())}>H₃</button
    >
    <button type="button" onclick={toggle(() => editor?.chain().focus().toggleBulletList().run())}
      >•</button
    >
    <button type="button" onclick={toggle(() => editor?.chain().focus().toggleOrderedList().run())}
      >1.</button
    >
    <button type="button" onclick={toggle(() => editor?.chain().focus().toggleBlockquote().run())}
      >"</button
    >
    <button type="button" onclick={toggle(() => editor?.chain().focus().toggleCode().run())}
      >&lt;&gt;</button
    >
    <button type="button" onclick={() => (linkPromptOpen = true)}>🔗</button>
  </div>
  <div bind:this={el} class="editor-host"></div>
  {#if saving}<span class="saving">Saving…</span>{/if}
</section>

<PromptDialog
  bind:open={linkPromptOpen}
  title="Insert link"
  placeholder="https://example.com"
  type="url"
  confirmLabel="Insert"
  onconfirm={(url) => {
    editor?.chain().focus().setLink({ href: url }).run();
    linkPromptOpen = false;
  }}
/>

<style>
  .about-editor {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .toolbar {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    border: 1px solid var(--border-subtle);
    border-bottom: 0;
    padding: 4px;
    background: var(--bg-elevated);
  }
  .toolbar button {
    background: transparent;
    border: 0;
    color: var(--fg-primary);
    padding: 4px 8px;
    font-family: var(--font-mono);
    font-size: 12px;
    cursor: pointer;
  }
  .toolbar button:hover {
    background: color-mix(in srgb, var(--accent) 12%, transparent);
  }
  .editor-host {
    border: 1px solid var(--border-subtle);
    background: var(--bg-canvas);
    min-height: 160px;
    padding: 12px;
  }
  .editor-host :global(.tiptap-bio) {
    outline: none;
    color: var(--fg-primary);
    font-family: inherit;
    line-height: 1.55;
  }
  .saving {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--fg-muted);
  }
</style>
