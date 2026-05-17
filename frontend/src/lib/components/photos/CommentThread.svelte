<script lang="ts">
  import { untrack } from 'svelte';
  import { page } from '$app/state';
  import { api, type Comment } from '$lib/api/client';
  import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';

  interface Props {
    photoId: string;
    photoOwnerId: string;
    initialCount: number;
    /** Fires whenever count changes from inside the thread so the parent can
        keep its own header/action-row label in sync. Optional. */
    oncountchange?: (n: number) => void;
  }

  let { photoId, photoOwnerId, initialCount, oncountchange }: Props = $props();

  let comments = $state<Comment[] | null>(null);
  let loadError = $state<string | null>(null);
  let draft = $state('');
  let posting = $state(false);
  let postError = $state<string | null>(null);
  let count = $state(untrack(() => initialCount));

  $effect(() => {
    oncountchange?.(count);
  });

  let viewer = $derived(page.data.user);
  let isOwner = $derived(viewer?.id === photoOwnerId);

  async function load() {
    loadError = null;
    try {
      const res = await api.comments.list(photoId);
      comments = res.comments;
      count = res.comments.length;
    } catch (e) {
      loadError = (e as Error).message;
    }
  }

  $effect(() => {
    void load();
  });

  async function post(e: SubmitEvent) {
    e.preventDefault();
    const body = draft.trim();
    if (body === '' || posting) return;
    posting = true;
    postError = null;
    try {
      const created = await api.comments.create(photoId, body);
      comments = [...(comments ?? []), created];
      count += 1;
      draft = '';
    } catch (err) {
      postError = (err as Error).message;
    } finally {
      posting = false;
    }
  }

  let deleteOpen = $state(false);
  let deleteTargetId = $state<string | null>(null);

  function askDelete(commentId: string) {
    deleteTargetId = commentId;
    deleteOpen = true;
  }

  async function performCommentDelete() {
    const commentId = deleteTargetId;
    if (!commentId) return;
    try {
      await api.comments.delete(commentId);
      comments = (comments ?? []).filter((c) => c.id !== commentId);
      count = Math.max(0, count - 1);
      deleteOpen = false;
      deleteTargetId = null;
    } catch (err) {
      postError = (err as Error).message;
      deleteOpen = false;
    }
  }

  function relTime(iso: string): string {
    const then = new Date(iso).getTime();
    const now = Date.now();
    const s = Math.max(1, Math.round((now - then) / 1000));
    if (s < 60) return `${s}s ago`;
    const m = Math.round(s / 60);
    if (m < 60) return `${m}m ago`;
    const h = Math.round(m / 60);
    if (h < 24) return `${h}h ago`;
    const d = Math.round(h / 24);
    if (d < 30) return `${d}d ago`;
    return new Date(iso).toLocaleDateString('en-GB', {
      day: '2-digit',
      month: 'short',
      year: 'numeric'
    });
  }

  function canDelete(c: Comment): boolean {
    if (!viewer) return false;
    if (isOwner) return true;
    return c.author_id === viewer.id;
  }
</script>

<section id="comments" class="thread" aria-label="Comments">
  <header class="head">
    <span class="t-label">COMMENTS</span>
    <span class="count t-mono">{count}</span>
  </header>

  {#if loadError}
    <p class="err">Couldn't load comments: {loadError}</p>
  {:else if comments === null}
    <p class="muted">Loading…</p>
  {:else if comments.length === 0}
    <p class="muted">No comments yet.</p>
  {:else}
    <ol class="list">
      {#each comments as c (c.id)}
        <li class="comment">
          <div class="meta">
            <span class="author">{c.author_display_name}</span>
            <span class="ts">{relTime(c.created_at)}</span>
            {#if canDelete(c)}
              <button type="button" class="del" onclick={() => askDelete(c.id)} aria-label="Delete">
                ×
              </button>
            {/if}
          </div>
          <p class="body">{c.body}</p>
        </li>
      {/each}
    </ol>
  {/if}

  {#if viewer}
    <form class="composer" onsubmit={post}>
      <textarea
        placeholder="Add a comment…"
        aria-label="Add a comment"
        bind:value={draft}
        rows="3"
        maxlength="2000"
        disabled={posting}
      ></textarea>
      <div class="row">
        {#if postError}<span class="err">{postError}</span>{/if}
        <button
          type="submit"
          class="btn btn-primary btn-sm"
          disabled={posting || draft.trim() === ''}
        >
          {posting ? 'Posting…' : 'Post comment'}
        </button>
      </div>
    </form>
  {:else}
    <p class="muted">
      <a href="/signin?return={encodeURIComponent(page.url.pathname + '#comments')}">Sign in</a> to comment.
    </p>
  {/if}
</section>

<ConfirmDialog
  bind:open={deleteOpen}
  title="Delete comment"
  message="Delete this comment? This cannot be undone."
  confirmLabel="Delete"
  tone="danger"
  onconfirm={performCommentDelete}
/>

<style>
  .thread {
    border-top: 1px solid var(--border-subtle);
    padding-top: 24px;
    margin-top: 32px;
  }
  .head {
    display: flex;
    align-items: baseline;
    gap: 8px;
    margin-bottom: 16px;
  }
  .count {
    color: var(--fg-muted);
    font-size: 11px;
  }
  .muted {
    color: var(--fg-muted);
    font-family: var(--font-mono);
    font-size: 12px;
  }
  .err {
    color: var(--danger, #c33);
    font-family: var(--font-mono);
    font-size: 12px;
  }
  .list {
    list-style: none;
    padding: 0;
    margin: 0 0 24px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .comment {
    padding-bottom: 12px;
    border-bottom: 1px solid var(--border-subtle);
  }
  .comment:last-child {
    border-bottom: 0;
  }
  .meta {
    display: flex;
    align-items: center;
    gap: 12px;
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--fg-muted);
    margin-bottom: 4px;
  }
  .author {
    color: var(--fg-primary);
    font-weight: 500;
  }
  .ts {
    flex: 1;
  }
  .del {
    background: transparent;
    color: var(--fg-muted);
    border: 1px solid var(--border-subtle);
    width: 22px;
    height: 22px;
    line-height: 1;
    cursor: pointer;
  }
  .del:hover {
    color: var(--danger, #c33);
    border-color: var(--danger, #c33);
  }
  .body {
    margin: 0;
    font-size: 14px;
    line-height: 1.55;
    color: var(--fg-primary);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .composer textarea {
    width: 100%;
    box-sizing: border-box;
    background: var(--bg-canvas);
    color: var(--fg-primary);
    border: 1px solid var(--border-subtle);
    padding: 10px 12px;
    font-family: var(--font-ui);
    font-size: 14px;
    resize: vertical;
  }
  .composer textarea:focus {
    outline: none;
    border-color: var(--accent);
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 12px;
    margin-top: 8px;
  }
  .row .err {
    flex: 1;
  }
</style>
