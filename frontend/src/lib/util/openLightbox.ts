import { preloadData, pushState } from '$app/navigation';
import type { Action } from 'svelte/action';

interface Options {
  short_id: string;
  handle: string;
}

export const openLightboxOnClick: Action<HTMLAnchorElement, Options> = (node, opts) => {
  let current = opts;

  function handler(e: MouseEvent) {
    if (e.button !== 0) return;
    if (e.metaKey || e.ctrlKey || e.altKey || e.shiftKey) return;
    e.preventDefault();
    void open();
  }

  async function open() {
    const url = `/u/${current.handle}/p/${current.short_id}`;
    const r = await preloadData(url);
    if (r.type !== 'loaded' || r.status !== 200) {
      window.location.href = url;
      return;
    }
    pushState(url, { lightbox: true, data: r.data });
  }

  node.addEventListener('click', handler);

  return {
    update(next: Options) {
      current = next;
    },
    destroy() {
      node.removeEventListener('click', handler);
    }
  };
};
