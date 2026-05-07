// Stub — Task 14 placeholder. Task 16 replaces this with the real Saver +
// debounced PATCH implementation. The shape here matches what VerifyPane
// expects so the component compiles.

export type SaveState = 'idle' | 'saving' | 'error';

type Snapshot = Record<string, unknown>;

export function useAutosave(_photoId: string) {
  let state = $state<SaveState>('idle');
  let label = $state<string>('');

  return {
    queue: (_snap: Snapshot) => {
      // Task 16 will implement debounced PATCH.
    },
    get state() {
      return state;
    },
    get label() {
      return label;
    },
    dispose: () => {},
  };
}
