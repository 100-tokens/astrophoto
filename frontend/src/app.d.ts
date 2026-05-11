declare global {
  namespace App {
    interface Locals {
      user: {
        id: string;
        email: string;
        displayName: string;
        handle: string;
        following_ids: string[];
        pending_deletion_at: string | null;
        tier: 'free' | 'subscriber';
      } | null;
      preferences: { theme: 'dark' | 'light'; density: 'work' | 'data' };
    }
    interface PageData {
      user: {
        id: string;
        email: string;
        displayName: string;
        handle: string;
        following_ids: string[];
        pending_deletion_at: string | null;
        tier: 'free' | 'subscriber';
      } | null;
    }
  }
}

export {};
