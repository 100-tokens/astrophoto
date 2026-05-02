declare global {
  namespace App {
    interface Locals {
      user: {
        id: string;
        displayName: string;
        following_ids: string[];
      } | null;
      preferences: { theme: 'dark' | 'light'; density: 'work' | 'data' };
    }
    interface PageData {
      user: { id: string; displayName: string; following_ids: string[] } | null;
    }
  }
}

export {};
