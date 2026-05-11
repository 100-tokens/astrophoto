<script lang="ts">
  import { invalidateAll } from '$app/navigation';
  import type { Comment } from '$lib/api/client';
  import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';

  interface Props {
    photoOwnerId: string;
    comments: Comment[];
    currentUser: { id: string; displayName: string } | null;
  }

  let { photoOwnerId, comments, currentUser }: Props = $props();

  let body = $state('');
  let posting = $state(false);
  let error = $state<string | null>(null);

  async function postComment() {
    if (posting || body.trim().length === 0) return;
    posting = true;
    error = null;
    try {
      const form = new FormData();
      form.append('body', body);
      const res = await fetch(`?/comment`, {
        method: 'POST',
        body: form
      });
      if (!res.ok) {
        error = 'Failed to post comment.';
        return;
      }
      body = '';
      await invalidateAll();
    } finally {
      posting = false;
    }
  }

  let deleteOpen = $state(false);
  let deleteTargetId = $state<string | null>(null);

  function askDeleteComment(commentId: string) {
    deleteTargetId = commentId;
    deleteOpen = true;
  }

  async function deleteComment() {
    const commentId = deleteTargetId;
    if (!commentId) return;
    const form = new FormData();
    form.append('id', commentId);
    const res = await fetch(`?/deleteComment`, {
      method: 'POST',
      body: form
    });
    if (res.ok) {
      await invalidateAll();
    }
    deleteOpen = false;
    deleteTargetId = null;
  }

  function timeAgo(iso: string): string {
    const d = new Date(iso);
    const seconds = (Date.now() - d.getTime()) / 1000;
    if (seconds < 60) return 'just now';
    if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
    if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
    if (seconds < 7 * 86400) return `${Math.floor(seconds / 86400)}d ago`;
    return d.toISOString().slice(0, 10);
  }

  function canDelete(c: Comment): boolean {
    if (!currentUser) return false;
    return c.author_id === currentUser.id || photoOwnerId === currentUser.id;
  }
</script>

<section class="comments">
  <div class="t-eyebrow comments-header">
    COMMENTS · {comments.length}
  </div>

  {#if comments.length === 0}
    <p class="empty">No comments yet.</p>
  {/if}

  {#each comments as c (c.id)}
    <div class="comment">
      <div class="meta">
        <span class="author">
          {c.author_display_name}{#if currentUser && c.author_id === currentUser.id}<span
              class="you"
            >
              · You</span
            >{/if}
        </span>
        <span class="time">{timeAgo(c.created_at)}</span>
      </div>
      <p class="body">{c.body}</p>
      {#if canDelete(c)}
        <button type="button" class="delete" onclick={() => askDeleteComment(c.id)}>
          Delete
        </button>
      {/if}
    </div>
  {/each}

  {#if currentUser}
    <form
      method="POST"
      class="composer"
      onsubmit={(e) => {
        e.preventDefault();
        postComment();
      }}
    >
      <textarea bind:value={body} placeholder="Add a comment..." rows="3" maxlength="2000"
      ></textarea>
      {#if error}
        <p class="error">{error}</p>
      {/if}
      <button type="submit" class="post" disabled={posting || body.trim().length === 0}>
        {posting ? 'Posting...' : 'Post'}
      </button>
    </form>
  {:else}
    <p class="signin-prompt">
      <a href="/signin">Sign in</a> to comment.
    </p>
  {/if}
</section>

<ConfirmDialog
  bind:open={deleteOpen}
  title="Delete comment"
  message="Delete this comment? This cannot be undone."
  confirmLabel="Delete"
  tone="danger"
  onconfirm={deleteComment}
/>

<style>
  .comments {
    padding: 24px 32px 32px;
    border-top: 1px solid var(--border-default);
  }
  .comments-header {
    color: var(--fg-primary);
    letter-spacing: 0.16em;
    margin-bottom: 16px;
  }
  .empty {
    color: var(--fg-muted);
    font-size: 13px;
    padding: 8px 0;
  }
  .comment {
    padding: 12px 0;
    border-bottom: 1px dashed var(--border-subtle);
  }
  .comment:last-child {
    border-bottom: 0;
  }
  .meta {
    display: flex;
    justify-content: space-between;
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--fg-muted);
    margin-bottom: 6px;
  }
  .author {
    color: var(--fg-primary);
  }
  .you {
    color: var(--accent);
  }
  .body {
    margin: 0;
    font-size: 14px;
    line-height: 1.55;
    color: var(--fg-secondary);
    white-space: pre-wrap;
    word-wrap: break-word;
  }
  .delete {
    margin-top: 6px;
    background: transparent;
    border: 0;
    padding: 0;
    color: var(--danger);
    font-family: var(--font-mono);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    cursor: pointer;
  }
  .delete:hover {
    text-decoration: underline;
  }
  .composer {
    margin-top: 16px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  textarea {
    width: 100%;
    background: var(--bg-base);
    color: var(--fg-primary);
    border: 1px solid var(--border-default);
    border-radius: 2px;
    padding: 8px 12px;
    font-family: var(--font-ui);
    font-size: 14px;
    line-height: 1.55;
    resize: vertical;
  }
  textarea:focus {
    outline: 0;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px rgba(232, 164, 58, 0.12);
  }
  .post {
    align-self: flex-end;
    background: var(--accent);
    color: var(--accent-ink);
    border: 0;
    padding: 0 16px;
    height: 32px;
    border-radius: 2px;
    font-family: var(--font-ui);
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }
  .post:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
  .error {
    color: var(--danger);
    font-size: 12px;
    margin: 0;
  }
  .signin-prompt {
    margin-top: 16px;
    color: var(--fg-muted);
    font-size: 13px;
  }
  .signin-prompt a {
    color: var(--accent);
  }
</style>
